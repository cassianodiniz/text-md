import Testing
import Foundation
import CSQLite
@testable import TextMDCore

@Suite(.serialized)
final class PersistenceTests {
    private var root: URL!
    init() throws {
        root = FileManager.default.temporaryDirectory.appendingPathComponent("text-md-tests-" + UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    }
    deinit { try? FileManager.default.removeItem(at: root) }
    private func file(_ name: String, _ text: String) throws -> URL {
        let url = root.appendingPathComponent(name)
        try Data(text.utf8).write(to: url)
        return url
    }
    private var library: URL { root.appendingPathComponent("library") }

    @Test func testFileIdentityRecognizesAliasesWithoutRewritingStoredPath() throws {
        let original = try file("documento.md", "# Original")
        let alias = root.appendingPathComponent("atalho.md")
        try FileManager.default.createSymbolicLink(at: alias, withDestinationURL: original)
        #expect(FileService.documentKey(original) == FileService.documentKey(alias))
        #expect(try FileService.open(alias).sourcePath == alias.standardizedFileURL.path)
    }

    @Test func testNewEmptyAndWhitespaceDraftsAreNotPersisted() throws {
        let store = try DraftStore(root: library)
        try store.save(Draft())
        try store.save(Draft(content: " \t\r\n\n"))
        #expect(try store.allDrafts().isEmpty)
    }

    @Test func testWrittenDraftStillSurvivesAfterItsTextIsCleared() throws {
        let store = try DraftStore(root: library)
        var draft = Draft(content: "Uma ideia escrita")
        try store.save(draft)
        let revision = Revision(draftID: draft.id, content: draft.content, reason: "Antes de apagar")
        draft.content = ""
        try store.save(draft, snapshot: revision)
        #expect(try store.allDrafts().first?.content == "")
        #expect(try store.revisions(for: draft.id).first?.content == "Uma ideia escrita")
    }

    @Test func testLegacyBlankCleanupIsRecoverableAndPreservesWrittenHistoryAndFiles() throws {
        let store = try DraftStore(root: library)
        var empty = Draft(); empty.hasWrittenContent = nil
        var written = Draft(content: "Conteúdo anterior"); written.hasWrittenContent = nil
        var expiredHistory = Draft(); expiredHistory.hasWrittenContent = nil
        expiredHistory.lastSnapshotAt = Date().addingTimeInterval(-100 * 86400)
        try store.save(written, snapshot: Revision(draftID: written.id, content: written.content, reason: "Anterior"))
        written.content = ""
        let original = try file("vazio.md", "")
        let opened = try FileService.open(original); try store.save(opened)
        // Fixture from v0.1: that version persisted blanks without this policy.
        var db: OpaquePointer?
        #expect(sqlite3_open(library.appendingPathComponent("library.sqlite").path, &db) == SQLITE_OK)
        defer { sqlite3_close(db) }
        for draft in [empty, written, expiredHistory] {
            let hex = try JSONEncoder().encode(draft).map { String(format: "%02x", $0) }.joined()
            #expect(sqlite3_exec(db, "INSERT OR REPLACE INTO drafts VALUES ('\(draft.id.uuidString)', X'\(hex)');", nil, nil, nil) == SQLITE_OK)
        }
        #expect(try store.recoverLegacyEmptyDrafts() == [empty.id])
        let values = try store.allDrafts()
        #expect(values.first { $0.id == empty.id }?.deletedAt != nil)
        #expect(values.first { $0.id == written.id }?.deletedAt == nil)
        #expect(values.first { $0.id == written.id }?.hasWrittenContent == true)
        #expect(values.first { $0.id == expiredHistory.id }?.deletedAt == nil)
        #expect(values.first { $0.id == opened.id }?.deletedAt == nil)
        #expect(try Data(contentsOf: original).isEmpty)
        #expect(try store.recoverLegacyEmptyDrafts().isEmpty)
    }

    @Test func testCommittedDraftAndSessionSurviveReopening() throws {
        let draft = Draft(content: "# Post\n\nTexto com acentuação 😀\n")
        do {
            let store = try DraftStore(root: library)
            try store.save(draft)
            var session = SessionState(); session.tabs = [draft.id]; session.selectedID = draft.id
            session.sourceMode = true; session.showLineNumbers = true
            session.folders = [PinnedFolder(path: root.path)]
            try store.saveSession(session)
        }
        let recovered = try DraftStore(root: library)
        #expect((try recovered.allDrafts().first) == ( draft))
        #expect((try recovered.session().selectedID) == ( draft.id))
        #expect(try recovered.session().showLineNumbers)
        #expect((try recovered.session().folders.first?.path) == ( root.path))
    }
    @Test func testEditingDraftDoesNotTouchOriginal() throws {
        let original = try file("SKILL.md", "---\nname: exemplo\n---\n\n# Original\n")
        var draft = try FileService.open(original)
        draft.content += "Rascunho novo\n"
        let store = try DraftStore(root: library); try store.save(draft)
        #expect((try String(contentsOf: original)) == ( "---\nname: exemplo\n---\n\n# Original\n"))
        #expect(draft.isModified)
        #expect((FileService.diskState(for: draft)) == ( .unchanged))
    }
    @Test func testExplicitSaveUpdatesOriginalAndBaseline() throws {
        let original = try file("post.md", "Antes")
        var draft = try FileService.open(original); draft.content = "Depois"
        let result = try FileService.save(draft, assetsRoot: library.appendingPathComponent("assets"))
        #expect((try String(contentsOf: original)) == ( "Depois"))
        #expect(!(result.isModified))
        #expect((FileService.diskState(for: result)) == ( .unchanged))
    }
    @Test func testExternalChangeIsDetectedEvenWithSameModificationDate() throws {
        let original = try file("post.md", "Original")
        let attributes = try FileManager.default.attributesOfItem(atPath: original.path)
        var draft = try FileService.open(original); draft.content = "Meu rascunho"
        try Data("Alterado fora".utf8).write(to: original)
        if let date = attributes[.modificationDate] as? Date {
            try FileManager.default.setAttributes([.modificationDate: date], ofItemAtPath: original.path)
        }
        #expect((FileService.diskState(for: draft)) == ( .changed("Alterado fora")))
        #expect(throws: (any Error).self) { try FileService.save(draft, assetsRoot: library) }
        #expect((try String(contentsOf: original)) == ( "Alterado fora"))
        #expect((draft.content) == ( "Meu rascunho"))
    }
    @Test func testSavingCopyAfterConflictPreservesBothTexts() throws {
        let original = try file("post.md", "Original")
        var draft = try FileService.open(original); draft.content = "Meu rascunho"
        try Data("Versão externa".utf8).write(to: original)
        let copy = root.appendingPathComponent("post-copia.md")
        let saved = try FileService.save(draft, to: copy, assetsRoot: library)
        #expect((try String(contentsOf: original)) == ( "Versão externa"))
        #expect((try String(contentsOf: copy)) == ( "Meu rascunho"))
        #expect((saved.sourcePath) == ( copy.path))
    }
    @Test func testMissingOriginalCannotBeSilentlyRecreated() throws {
        let original = try file("post.md", "Original")
        var draft = try FileService.open(original); draft.content = "Meu rascunho"
        try FileManager.default.removeItem(at: original)
        #expect((FileService.diskState(for: draft)) == ( .missing))
        #expect(throws: (any Error).self) { try FileService.save(draft, assetsRoot: library) }
        #expect(!(FileManager.default.fileExists(atPath: original.path)))
    }
    @Test func testCopyDoesNotOverwriteExistingFileWithoutExplicitPermission() throws {
        let destination = try file("copia.md", "Preservar")
        #expect(throws: (any Error).self) { try FileService.save(Draft(content: "Novo"), to: destination, assetsRoot: library) }
        #expect((try String(contentsOf: destination)) == ( "Preservar"))
    }
    @Test func testInvalidDestinationDoesNotLoseStoredDraft() throws {
        let store = try DraftStore(root: library)
        let draft = Draft(content: "Trabalho recuperável"); try store.save(draft)
        #expect(throws: (any Error).self) { try FileService.save(draft, to: root.appendingPathComponent("ausente/post.md"), assetsRoot: store.assetsRoot) }
        #expect((try store.allDrafts().first?.content) == ( draft.content))
    }
    @Test func testReadOnlyStorageReportsFailureAndPreservesLastCommit() throws {
        let store = try DraftStore(root: library)
        var draft = Draft(content: "Guardado"); try store.save(draft)
        let readOnly = try DraftStore(root: library, readOnly: true)
        draft.content = "Não confirmado"
        #expect(throws: (any Error).self) { try readOnly.save(draft) }
        #expect((try store.allDrafts().first?.content) == ( "Guardado"))
    }
    @Test func testRestoreSnapshotCanPreserveVersionBeingReplaced() throws {
        let store = try DraftStore(root: library)
        var draft = Draft(content: "Versão inicial"); try store.save(draft)
        let old = Revision(draftID: draft.id, content: draft.content, reason: "Antes de editar")
        draft.content = "Versão seguinte"; try store.save(draft, snapshot: old)
        let snapshot = try #require(store.revisions(for: draft.id).first)
        let replacement = Revision(draftID: draft.id, content: draft.content, reason: "Antes de restaurar")
        draft.content = snapshot.content; try store.save(draft, snapshot: replacement)
        #expect((try store.allDrafts().first?.content) == ( "Versão inicial"))
        #expect((try store.revisions(for: draft.id).count) == ( 2))
    }
    @Test func testCleanupOnlyExpiresInternalTrashAndOldHistoryAfter90Days() throws {
        let now = Date(timeIntervalSince1970: 1_800_000_000)
        let store = try DraftStore(root: library)
        let original = try file("original.md", "Original intacto")
        let oldDate = now.addingTimeInterval(-91 * 86400)
        var active = try FileService.open(original, now: oldDate); active.content = "Rascunho antigo ativo"
        var expired = Draft(content: "Lixeira antiga", now: oldDate); expired.deletedAt = oldDate
        var recent = Draft(content: "Lixeira recente"); recent.deletedAt = now.addingTimeInterval(-89 * 86400)
        var boundary = Draft(content: "Exatamente 90 dias"); boundary.deletedAt = now.addingTimeInterval(-90 * 86400)
        try store.save(active, snapshot: Revision(draftID: active.id, content: "Antiga", reason: "teste", now: oldDate))
        try store.save(expired); try store.save(recent); try store.save(boundary)
        #expect((try store.cleanup(now: now)) == ( [expired.id]))
        let ids = Set(try store.allDrafts().map(\.id))
        #expect(ids.contains(active.id)); #expect(ids.contains(recent.id)); #expect(ids.contains(boundary.id))
        #expect((try store.revisions(for: active.id).count) == ( 0))
        #expect((try String(contentsOf: original)) == ( "Original intacto"))
    }
    @Test func testImagesArePrivateUntilSaveThenCopiedWithRelativeReference() throws {
        let store = try DraftStore(root: library)
        let bytes = Data([1, 2, 3, 4])
        let ref = try FileService.storeImage(bytes, extension: "png", assetsRoot: store.assetsRoot)
        let draft = Draft(content: "# Post\n\n![Exemplo](\(ref))\n")
        let destination = root.appendingPathComponent("Meu post.md")
        #expect(!(FileManager.default.fileExists(atPath: root.appendingPathComponent("Meu post.images").path)))
        let saved = try FileService.save(draft, to: destination, assetsRoot: store.assetsRoot)
        #expect(!(saved.content.contains("textmd-asset:")))
        #expect(saved.content.contains("Meu%20post.images/"))
        let names = try FileManager.default.contentsOfDirectory(atPath: root.appendingPathComponent("Meu post.images").path)
        #expect((names.count) == ( 1))
        #expect((try Data(contentsOf: root.appendingPathComponent("Meu post.images/" + names[0]))) == ( bytes))
        #expect(FileManager.default.fileExists(atPath: try FileService.assetURL(ref, assetsRoot: store.assetsRoot).path))
    }
    @Test func testMissingImagePreventsReplacingOriginal() throws {
        let original = try file("post.md", "Original")
        var draft = try FileService.open(original); draft.content = "![foto](textmd-asset:missing.png)"
        #expect(throws: (any Error).self) { try FileService.save(draft, assetsRoot: library) }
        #expect((try String(contentsOf: original)) == ( "Original"))
    }
    @Test func testAssetPathTraversalIsRejected() throws {
        #expect(throws: (any Error).self) { try FileService.assetURL("textmd-asset:../original.md", assetsRoot: library) }
        #expect(throws: (any Error).self) { try FileService.assetURL("textmd-asset:foo\\bar.png", assetsRoot: library) }
    }
    @Test func testOpeningAndSavingUneditedUTF8PreservesBytesIncludingBOMAndCRLF() throws {
        let original = root.appendingPathComponent("SKILL.md")
        let bytes = Data([0xef, 0xbb, 0xbf]) + Data("---\r\nname: exemplo\r\n---\r\n\r\n# Olá\r\n".utf8)
        try bytes.write(to: original)
        let draft = try FileService.open(original)
        #expect(!(draft.isModified))
        _ = try FileService.save(draft, assetsRoot: library)
        #expect((try Data(contentsOf: original)) == ( bytes))
    }
    @Test func testMovingDocumentPreservesBytesAndRelativeImages() throws {
        let image = try file("foto.png", "imagem de teste")
        let original = try file("post.md", "# Post\n\n![Foto](foto.png)\n")
        let bytes = try Data(contentsOf: original)
        let folder = root.appendingPathComponent("Destino")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let target = folder.appendingPathComponent("renomeado.md")
        try FileService.moveDocument(from: original, to: target)
        #expect(try Data(contentsOf: target) == bytes)
        #expect(try Data(contentsOf: folder.appendingPathComponent("foto.png")) == Data(contentsOf: image))
        #expect(!FileManager.default.fileExists(atPath: original.path))
        #expect(FileManager.default.fileExists(atPath: image.path))
    }
    @Test func testMovingCannotReplaceExistingDocumentOrImage() throws {
        _ = try file("foto.png", "imagem original")
        let original = try file("post.md", "![Foto](foto.png)")
        let folder = root.appendingPathComponent("Destino")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let image = folder.appendingPathComponent("foto.png")
        try Data("outra imagem".utf8).write(to: image)
        let target = folder.appendingPathComponent("post.md")
        #expect(throws: (any Error).self) { try FileService.moveDocument(from: original, to: target) }
        #expect(try String(contentsOf: original) == "![Foto](foto.png)")
        #expect(try String(contentsOf: image) == "outra imagem")
        try Data("arquivo existente".utf8).write(to: target)
        #expect(throws: (any Error).self) { try FileService.moveDocument(from: original, to: target) }
        #expect(try String(contentsOf: target) == "arquivo existente")
    }
    @Test func testSaveCopyKeepsExistingRelativeImagesUsable() throws {
        _ = try file("foto.png", "imagem original")
        let original = try file("post.md", "![Foto](foto.png)")
        let draft = try FileService.open(original)
        let folder = root.appendingPathComponent("Destino")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let copy = folder.appendingPathComponent("copia.md")
        _ = try FileService.save(draft, to: copy, assetsRoot: library)
        #expect(try String(contentsOf: copy) == draft.content)
        #expect(try String(contentsOf: folder.appendingPathComponent("foto.png")) == "imagem original")
        #expect(FileManager.default.fileExists(atPath: original.path))
    }
}
