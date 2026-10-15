import Foundation
import CSQLite

/// SQLite commits each edit before the UI can claim it is stored. FULL durability
/// and a transaction keep the draft and its recovery snapshot together.
public final class DraftStore {
    private var db: OpaquePointer?
    public let root: URL
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()

    public init(root: URL, readOnly: Bool = false) throws {
        self.root = root
        if !readOnly { try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true) }
        let flags = readOnly ? SQLITE_OPEN_READONLY : SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE
        guard sqlite3_open_v2(root.appendingPathComponent("library.sqlite").path, &db, flags, nil) == SQLITE_OK else {
            throw TextMDError.storage("Não foi possível abrir a biblioteca local.")
        }
        sqlite3_busy_timeout(db, 5000)
        if !readOnly {
            try execute("PRAGMA journal_mode=WAL;")
            try execute("PRAGMA synchronous=FULL;")
            try execute("CREATE TABLE IF NOT EXISTS drafts (id TEXT PRIMARY KEY, json BLOB NOT NULL);")
            try execute("CREATE TABLE IF NOT EXISTS revisions (id TEXT PRIMARY KEY, draft_id TEXT NOT NULL, created REAL NOT NULL, json BLOB NOT NULL);")
            try execute("CREATE INDEX IF NOT EXISTS revisions_draft ON revisions(draft_id, created);")
            try execute("CREATE TABLE IF NOT EXISTS settings (id TEXT PRIMARY KEY, json BLOB NOT NULL);")
        }
    }
    deinit { sqlite3_close(db) }

    private func error() -> TextMDError {
        .storage(db.map { String(cString: sqlite3_errmsg($0)) } ?? "Biblioteca fechada.")
    }
    private func execute(_ sql: String) throws {
        guard sqlite3_exec(db, sql, nil, nil, nil) == SQLITE_OK else { throw error() }
    }
    private func statement(_ sql: String) throws -> OpaquePointer {
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK, let stmt else { throw error() }
        return stmt
    }
    private func bind(_ value: String, to stmt: OpaquePointer, index: Int32) {
        _ = value.withCString { sqlite3_bind_text(stmt, index, $0, -1, unsafeBitCast(-1, to: sqlite3_destructor_type.self)) }
    }
    private func bind<T: Encodable>(_ value: T, to stmt: OpaquePointer, index: Int32) throws {
        let data = try encoder.encode(value)
        _ = data.withUnsafeBytes { sqlite3_bind_blob(stmt, index, $0.baseAddress, Int32(data.count), unsafeBitCast(-1, to: sqlite3_destructor_type.self)) }
    }
    private func step(_ stmt: OpaquePointer) throws {
        guard sqlite3_step(stmt) == SQLITE_DONE else { throw error() }
    }
    private func decode<T: Decodable>(_ type: T.Type, from stmt: OpaquePointer, column: Int32 = 0) throws -> T {
        guard let bytes = sqlite3_column_blob(stmt, column) else { throw TextMDError.storage("Registro incompleto.") }
        return try decoder.decode(type, from: Data(bytes: bytes, count: Int(sqlite3_column_bytes(stmt, column))))
    }
    private func transaction(_ body: () throws -> Void) throws {
        try execute("BEGIN IMMEDIATE;")
        do { try body(); try execute("COMMIT;") }
        catch { try? execute("ROLLBACK;"); throw error }
    }

    public func allDrafts() throws -> [Draft] {
        let stmt = try statement("SELECT json FROM drafts;"); defer { sqlite3_finalize(stmt) }
        var values: [Draft] = []
        while true {
            let status = sqlite3_step(stmt)
            if status == SQLITE_DONE { break }
            guard status == SQLITE_ROW else { throw error() }
            values.append(try decode(Draft.self, from: stmt))
        }
        return values.sorted { $0.modifiedAt > $1.modifiedAt }
    }
    public func save(_ draft: Draft, snapshot: Revision? = nil) throws {
        try save(draft, snapshots: snapshot.map { [$0] } ?? [])
    }
    public func save(_ draft: Draft, snapshots: [Revision]) throws {
        guard draft.shouldPersist else { return }
        try transaction {
            for snapshot in snapshots { try insertRevision(snapshot) }
            try upsert(draft)
        }
    }
    private func upsert(_ draft: Draft) throws {
            let stmt = try statement("INSERT INTO drafts(id,json) VALUES (?,?) ON CONFLICT(id) DO UPDATE SET json=excluded.json;")
            defer { sqlite3_finalize(stmt) }
            bind(draft.id.uuidString, to: stmt, index: 1)
            try bind(draft, to: stmt, index: 2); try step(stmt)
    }
    public func saveClosing(_ drafts: [Draft], session: SessionState) throws {
        try transaction {
            for draft in drafts where draft.shouldPersist { try upsert(draft) }
            try saveSession(session)
        }
    }
    private func insertRevision(_ revision: Revision) throws {
        let stmt = try statement("INSERT INTO revisions(id,draft_id,created,json) VALUES (?,?,?,?);")
        defer { sqlite3_finalize(stmt) }
        bind(revision.id.uuidString, to: stmt, index: 1)
        bind(revision.draftID.uuidString, to: stmt, index: 2)
        sqlite3_bind_double(stmt, 3, revision.createdAt.timeIntervalSince1970)
        try bind(revision, to: stmt, index: 4); try step(stmt)
    }
    public func revisions(for id: UUID) throws -> [Revision] {
        let stmt = try statement("SELECT json FROM revisions WHERE draft_id=? ORDER BY created DESC;")
        defer { sqlite3_finalize(stmt) }; bind(id.uuidString, to: stmt, index: 1)
        var values: [Revision] = []
        while true {
            let status = sqlite3_step(stmt)
            if status == SQLITE_DONE { break }
            guard status == SQLITE_ROW else { throw error() }
            values.append(try decode(Revision.self, from: stmt))
        }
        return values
    }
    /// Migrate only old, untouched, app-owned blanks. Written notes and real files
    /// stay intact; a recoverable trash row is used instead of deleting data.
    @discardableResult public func recoverLegacyEmptyDrafts(now: Date = Date()) throws -> [UUID] {
        var moved: [UUID] = []
        for var draft in try allDrafts() where draft.sourcePath == nil && draft.deletedAt == nil && draft.hasWrittenContent == nil {
            let writtenHistory = try revisions(for: draft.id).contains { !$0.content.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
            if draft.lastSnapshotAt != nil || !draft.content.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ||
                writtenHistory {
                draft.hasWrittenContent = true
            } else {
                draft.deletedAt = now; draft.hasWrittenContent = false
                moved.append(draft.id)
            }
            try save(draft)
        }
        return moved
    }
    /// Older versions listed every stored note, even after its tab was closed.
    /// Hide those entries without deleting their text, history or source files.
    public func hideLegacyClosedDrafts(session: SessionState) throws {
        let openIDs = Set(session.windows?.flatMap(\.tabs) ?? session.tabs)
        for var draft in try allDrafts() where draft.deletedAt == nil && draft.hiddenFromSidebar == nil {
            draft.hiddenFromSidebar = !openIDs.contains(draft.id) && !draft.pinned
            try save(draft)
        }
    }
    /// Keep every divergent legacy draft accessible instead of silently hiding
    /// its placement behind another draft pointing at the same file.
    public func recoverDuplicateSources() throws {
        var seen = Set<String>()
        let values = try allDrafts()
        try transaction {
            for var draft in values where draft.deletedAt == nil {
                guard let path = draft.sourcePath else { continue }
                guard !seen.insert(FileService.documentKey(URL(fileURLWithPath: path))).inserted else { continue }
                draft.displayName = draft.title + " — recuperado"
                draft.referenceBasePath = draft.referencePath
                draft.sourcePath = nil; draft.baselineHash = nil; draft.bookmark = nil
                draft.hasWrittenContent = true; draft.hiddenFromSidebar = false
                try upsert(draft)
            }
        }
    }
    public func saveSession(_ session: SessionState) throws {
        let stmt = try statement("INSERT INTO settings(id,json) VALUES ('session',?) ON CONFLICT(id) DO UPDATE SET json=excluded.json;")
        defer { sqlite3_finalize(stmt) }; try bind(session, to: stmt, index: 1); try step(stmt)
    }
    public func session() throws -> SessionState {
        let stmt = try statement("SELECT json FROM settings WHERE id='session';")
        defer { sqlite3_finalize(stmt) }
        let status = sqlite3_step(stmt)
        if status == SQLITE_DONE { return SessionState() }
        guard status == SQLITE_ROW else { throw error() }
        return try decode(SessionState.self, from: stmt)
    }
    /// Only app-owned rows expire. This never touches original files or macOS Trash.
    @discardableResult public func cleanup(now: Date = Date()) throws -> [UUID] {
        let cutoff = now.addingTimeInterval(-90 * 24 * 60 * 60)
        let expired = try allDrafts().filter { $0.deletedAt.map { $0 < cutoff } ?? false }
        try transaction {
            for draft in expired {
                let stmt = try statement("DELETE FROM drafts WHERE id=?;")
                bind(draft.id.uuidString, to: stmt, index: 1)
                try step(stmt); sqlite3_finalize(stmt)
                let versions = try statement("DELETE FROM revisions WHERE draft_id=?;")
                bind(draft.id.uuidString, to: versions, index: 1)
                try step(versions); sqlite3_finalize(versions)
            }
            let stmt = try statement("DELETE FROM revisions WHERE created < ?;")
            defer { sqlite3_finalize(stmt) }
            sqlite3_bind_double(stmt, 1, cutoff.timeIntervalSince1970); try step(stmt)
            var session = try self.session()
            let removed = Set(expired.map(\.id))
            session.tabs.removeAll { removed.contains($0) }
            if let id = session.selectedID, removed.contains(id) { session.selectedID = session.tabs.first }
            try saveSession(session)
        }
        // Only app-owned, old, unreferenced media can expire. A fresh orphan may
        // belong to an edit not yet committed, so it also receives the full grace period.
        try collectUnusedAssets(olderThan: cutoff)
        return expired.map(\.id)
    }
    private func collectUnusedAssets(olderThan cutoff: Date) throws {
        var texts = try allDrafts().map(\.content)
        let stmt = try statement("SELECT json FROM revisions;")
        defer { sqlite3_finalize(stmt) }
        while true {
            let status = sqlite3_step(stmt)
            if status == SQLITE_DONE { break }
            guard status == SQLITE_ROW else { throw error() }
            texts.append(try decode(Revision.self, from: stmt).content)
        }
        let regex = try NSRegularExpression(pattern: "textmd-asset:([A-Za-z0-9._-]+)")
        var used = Set<String>()
        for text in texts {
            let parsed = MarkdownAnalysis(text)
            for reference in parsed.images + parsed.links where reference.destination.hasPrefix("textmd-asset:") {
                if let url = try? FileService.assetURL(reference.destination, assetsRoot: assetsRoot) { used.insert(url.lastPathComponent) }
            }
            let ns = text as NSString
            for match in regex.matches(in: text, range: NSRange(location: 0, length: ns.length)) {
                used.insert(ns.substring(with: match.range(at: 1)))
            }
        }
        let fm = FileManager.default
        guard fm.fileExists(atPath: assetsRoot.path) else { return }
        for file in try fm.contentsOfDirectory(at: assetsRoot, includingPropertiesForKeys: [.isRegularFileKey, .isSymbolicLinkKey, .contentModificationDateKey]) {
            guard !used.contains(file.lastPathComponent), UUID(uuidString: file.deletingPathExtension().lastPathComponent) != nil,
                  ["png", "jpg", "jpeg", "gif", "tiff", "webp", "heic"].contains(file.pathExtension.lowercased()) else { continue }
            let values = try file.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .contentModificationDateKey])
            guard values.isRegularFile == true, values.isSymbolicLink != true,
                  let modified = values.contentModificationDate, modified < cutoff else { continue }
            try fm.removeItem(at: file)
        }
    }
    public var assetsRoot: URL { root.appendingPathComponent("assets", isDirectory: true) }
}
