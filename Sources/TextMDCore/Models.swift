import Foundation

public struct Draft: Codable, Identifiable, Equatable {
    public var id: UUID
    public var content: String
    public var sourcePath: String?
    public var baselineHash: String?
    public var bookmark: Data?
    public var hasUTF8BOM: Bool?
    public var createdAt: Date
    public var modifiedAt: Date
    public var deletedAt: Date?
    public var pinned: Bool
    public var selection: Int
    public var scrollPosition: Double
    public var lastSnapshotAt: Date?
    /// Optional for compatibility with the first version's library.
    public var hasWrittenContent: Bool?

    public init(id: UUID = UUID(), content: String = "", sourcePath: String? = nil,
                baselineHash: String? = nil, now: Date = Date()) {
        self.id = id; self.content = content; self.sourcePath = sourcePath
        self.baselineHash = baselineHash; createdAt = now; modifiedAt = now
        pinned = false; selection = 0; scrollPosition = 0
        hasWrittenContent = !content.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    public var title: String {
        if let sourcePath { return URL(fileURLWithPath: sourcePath).lastPathComponent }
        for line in content.components(separatedBy: .newlines) {
            let text = line.trimmingCharacters(in: .whitespaces)
                .replacingOccurrences(of: "^#{1,6}\\s+", with: "", options: .regularExpression)
            if !text.isEmpty { return String(text.prefix(64)) }
        }
        return "Novo rascunho"
    }
    public var isModified: Bool {
        guard let baselineHash else { return true }
        return FileService.hash(FileService.contentData(content, utf8BOM: hasUTF8BOM == true)) != baselineHash
    }
    public var shouldPersist: Bool {
        sourcePath != nil || deletedAt != nil || hasWrittenContent == true ||
        !content.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }
}

public struct Revision: Codable, Identifiable, Equatable {
    public var id: UUID
    public var draftID: UUID
    public var content: String
    public var createdAt: Date
    public var reason: String
    public init(draftID: UUID, content: String, reason: String, now: Date = Date()) {
        id = UUID(); self.draftID = draftID; self.content = content
        self.reason = reason; createdAt = now
    }
}

public struct PinnedFolder: Codable, Identifiable, Equatable {
    public var id: UUID
    public var path: String
    public var bookmark: Data?
    public init(path: String) { id = UUID(); self.path = path }
    public var title: String { URL(fileURLWithPath: path).lastPathComponent }
}

public struct SessionState: Codable, Equatable {
    public var tabs: [UUID] = []
    public var selectedID: UUID?
    public var folders: [PinnedFolder] = []
    public var sourceMode = false
    public var showLineNumbers = false
    public var expandedPaths: [String] = []
    // Optional fields let existing sessions decode without destructive resets.
    public var appearance: EditorAppearance?
    public var metrics: [DocumentMetric]?
    public var ignoreMarkdown: Bool?
    public var tokenProfile: TokenProfile?
    public var diffEnabled: Bool?
    public var zoom: Double?
    public var windows: [WindowState]?
    public var currentFolder: String?
    public var sidebarMode: SidebarMode?
    public var collapsedSections: [String]?
    public var sidebarVisible: Bool?
    public init() {}
}

public enum EditorAppearance: String, Codable, CaseIterable { case system, light, dark }
public enum DocumentMetric: String, Codable, CaseIterable { case lines, words, characters, tokens }
public enum TokenProfile: String, Codable, CaseIterable, Sendable {
    case o200k = "o200k_base", cl100k = "cl100k_base"
}

public enum DiskState: Equatable {
    case unchanged
    case changed(String)
    case missing
    case inaccessible(String)
}

public enum TextMDError: LocalizedError {
    case storage(String), conflict, missing, invalidUTF8, invalidDestination(String), invalidAsset
    public var errorDescription: String? {
        switch self {
        case .storage(let message): return "Não foi possível guardar o rascunho: \(message)"
        case .conflict: return "O original mudou por fora. Compare os textos antes de salvar."
        case .missing: return "O arquivo original não está mais nesse lugar. Seu rascunho está preservado."
        case .invalidUTF8: return "Este arquivo não está em UTF-8. Ele foi preservado sem alterações."
        case .invalidDestination(let message): return "Não foi possível salvar: \(message)"
        case .invalidAsset: return "A referência de imagem é inválida ou a imagem não está disponível."
        }
    }
}
