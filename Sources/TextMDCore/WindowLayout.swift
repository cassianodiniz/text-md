import Foundation

public enum SidebarMode: String, Codable { case documents, folders }

public struct WindowState: Codable, Identifiable, Equatable {
    public var id: UUID
    public var tabs: [UUID]
    public var selectedID: UUID?
    public var sourceMode: Bool
    public var showLineNumbers: Bool
    public var diffEnabled: Bool?
    public var currentFolder: String?
    public var sidebarMode: SidebarMode?
    public var collapsedSections: [String]?
    public var sidebarVisible: Bool?
    public init(id: UUID = UUID(), session: SessionState = SessionState()) {
        self.id = id; tabs = session.tabs; selectedID = session.selectedID
        sourceMode = session.sourceMode; showLineNumbers = session.showLineNumbers; diffEnabled = session.diffEnabled
        currentFolder = session.currentFolder; sidebarMode = session.sidebarMode
        collapsedSections = session.collapsedSections; sidebarVisible = session.sidebarVisible
    }
    public func applying(to preferences: SessionState) -> SessionState {
        var value = preferences
        value.tabs = tabs; value.selectedID = selectedID
        value.sourceMode = sourceMode; value.showLineNumbers = showLineNumbers; value.diffEnabled = diffEnabled
        value.currentFolder = currentFolder; value.sidebarMode = sidebarMode
        value.collapsedSections = collapsedSections; value.sidebarVisible = sidebarVisible
        return value
    }
}

/// One owner per document. Window moves change placement, never content or identity.
public struct WindowLayout {
    public var windows: [WindowState]
    public init(windows: [WindowState], validIDs: Set<UUID>? = nil, identities: [UUID: String] = [:]) {
        var seen = Set<String>()
        self.windows = windows.map { state in
            var state = state
            state.tabs = state.tabs.filter { (validIDs?.contains($0) ?? true) && seen.insert(identities[$0] ?? $0.uuidString).inserted }
            if !state.tabs.contains(state.selectedID ?? UUID()) { state.selectedID = state.tabs.first }
            return state
        }
    }
    public func owner(of id: UUID) -> UUID? { windows.first { $0.tabs.contains(id) }?.id }
    @discardableResult public mutating func move(_ id: UUID, to target: UUID, before: UUID? = nil) -> Bool {
        guard windows.contains(where: { $0.id == target }), owner(of: id) != nil else { return false }
        if before == id && owner(of: id) == target { return true }
        for index in windows.indices {
            windows[index].tabs.removeAll { $0 == id }
            if windows[index].selectedID == id { windows[index].selectedID = windows[index].tabs.last }
        }
        let index = windows.firstIndex { $0.id == target }!
        let position = before.flatMap { windows[index].tabs.firstIndex(of: $0) } ?? windows[index].tabs.count
        windows[index].tabs.insert(id, at: position); windows[index].selectedID = id
        return true
    }
}
