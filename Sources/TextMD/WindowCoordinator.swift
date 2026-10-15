import AppKit
import Observation
import TextMDCore

@MainActor @Observable
final class AppLibrary {
    var drafts: [Draft] = []
    var preferences = SessionState()
    var uncommitted: Set<UUID> = []
    var errorMessage: String?
    var sessionWriteFailed = false
    @ObservationIgnored var undoManagers: [UUID: UndoManager] = [:]
    @ObservationIgnored var pendingSnapshots: [UUID: [Revision]] = [:]
    @ObservationIgnored var editorScrollViews: [UUID: NSScrollView] = [:]
    func discardEditor(_ id: UUID) {
        if let editor = editorScrollViews.removeValue(forKey: id)?.documentView as? MarkdownTextView {
            if editor.model?.editor === editor { editor.model?.editor = nil }
            if let observer = editor.boundsObserver { NotificationCenter.default.removeObserver(observer); editor.boundsObserver = nil }
        }
        undoManagers.removeValue(forKey: id)?.removeAllActions()
    }
    @ObservationIgnored var store: DraftStore?
    static func rootURL() throws -> URL {
        if let value = ProcessInfo.processInfo.environment["TEXTMD_LIBRARY_PATH"] { return URL(fileURLWithPath: value, isDirectory: true) }
        return try FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask,
            appropriateFor: nil, create: true).appendingPathComponent("text-md", isDirectory: true)
    }
    init(root: URL? = nil) {
        do {
            let store = try DraftStore(root: root ?? Self.rootURL()); self.store = store
            try store.cleanup(); try store.recoverLegacyEmptyDrafts()
            preferences = try store.session()
            try store.hideLegacyClosedDrafts(session: preferences)
            try store.recoverDuplicateSources()
            drafts = try store.allDrafts()
        } catch { self.store = nil; errorMessage = error.localizedDescription }
    }
}

@MainActor @Observable
final class WindowCoordinator: NSObject, NSWindowDelegate {
    static let shared = WindowCoordinator(library: AppLibrary())
    let library: AppLibrary
    private(set) var models: [AppModel] = []
    var activeWindowID: UUID?
    @ObservationIgnored var openWindowAction: (() -> Void)?
    @ObservationIgnored private var pending: [WindowState] = []
    @ObservationIgnored private var focusPending = Set<UUID>()
    @ObservationIgnored private var restored = false
    @ObservationIgnored var terminating = false
    @ObservationIgnored private var pendingURLs: [URL] = []
    @ObservationIgnored private var pendingPanel: Bool?
    @ObservationIgnored private var closingWindowIDs = Set<UUID>()

    init(library: AppLibrary) {
        self.library = library
        super.init()
        let session = library.preferences
        let valid = Set(library.drafts.filter { $0.deletedAt == nil }.map(\.id))
        pending = WindowLayout(windows: session.windows ?? [WindowState(session: session)], validIDs: valid).windows
        library.preferences.closedItems = (session.closedItems ?? []).compactMap { item in
            var item = item
            item.window.tabs.removeAll { !valid.contains($0) }
            if !item.window.tabs.contains(item.window.selectedID ?? UUID()) { item.window.selectedID = item.window.tabs.first }
            return item.window.tabs.isEmpty ? nil : item
        }
    }
    var active: AppModel? {
        let available = models.filter { !closingWindowIDs.contains($0.windowID) }
        if let key = NSApp?.keyWindow, let model = available.first(where: { $0.nativeWindow === key }) { return model }
        return available.first { $0.windowID == activeWindowID } ?? available.last
    }
    var closedItems: [ClosedWorkspaceItem] { library.preferences.closedItems ?? [] }
    var lastClosedWindow: ClosedWorkspaceItem? { closedItems.last { $0.kind == .window } }
    var pendingWindowCount: Int { pending.count }

    func claimWindow() -> AppModel {
        let state = pending.isEmpty ? WindowState() : pending.removeFirst()
        let model = AppModel(library: library, state: state); model.coordinator = self
        if model.session.tabs.isEmpty { model.session.libraryVisible = true }
        models.append(model)
        return model
    }
    func attach(_ model: AppModel, to window: NSWindow) {
        model.nativeWindow = window; window.delegate = self
        window.identifier = NSUserInterfaceItemIdentifier(model.windowID.uuidString)
        window.setAccessibilityIdentifier(model.windowID.uuidString)
        window.tabbingMode = .disallowed
        if window.isKeyWindow || activeWindowID == nil { activeWindowID = model.windowID }
        model.checkDisk(); model.syncWindowTitle()
        if !restored {
            restored = true
            for _ in 0..<pending.count { openWindowAction?() }
        }
        if !pendingURLs.isEmpty {
            let urls = pendingURLs; pendingURLs = []
            for url in urls { model.open(url) }
        }
        persist(from: model)
        if focusPending.remove(model.windowID) != nil { focus(model) }
        if let folder = pendingPanel {
            pendingPanel = nil
            DispatchQueue.main.async { if folder { model.openFolderPanel() } else { model.openPanel() } }
        }
    }
    private func focus(_ model: AppModel) {
        activeWindowID = model.windowID
        if let window = model.nativeWindow {
            window.deminiaturize(nil); window.makeKeyAndOrderFront(nil); NSApp?.activate(ignoringOtherApps: true)
        }
    }
    func openPanel(folder: Bool) {
        if let model = active { if folder { model.openFolderPanel() } else { model.openPanel() } }
        else { pendingPanel = folder; createWindow(newDraft: false) }
    }
    func placement(of id: UUID) -> (AppModel, UUID)? {
        if let model = models.first(where: { !closingWindowIDs.contains($0.windowID) && $0.session.tabs.contains(id) }) { return (model, id) }
        guard let key = documentKey(id) else { return nil }
        for model in models where !closingWindowIDs.contains(model.windowID) {
            if let owned = model.session.tabs.first(where: { documentKey($0) == key }) { return (model, owned) }
        }
        return nil
    }
    private func documentKey(_ id: UUID) -> String? {
        library.drafts.first { $0.id == id }?.sourcePath.map { FileService.documentKey(URL(fileURLWithPath: $0)) }
    }
    /// A queued window reserves its documents before its NSWindow exists.
    func routePendingDocument(_ id: UUID) -> Bool {
        let key = documentKey(id)
        for index in pending.indices {
            guard let owned = pending[index].tabs.first(where: { $0 == id || (key != nil && documentKey($0) == key) }) else { continue }
            pending[index].selectedID = owned; pending[index].libraryVisible = false
            focusPending.insert(pending[index].id)
            return true
        }
        return false
    }
    func owner(of id: UUID) -> AppModel? { placement(of: id)?.0 }
    func createWindow(newDraft: Bool = true) {
        var state = WindowState()
        if newDraft {
            let draft = Draft(); library.drafts.append(draft)
            state.tabs = [draft.id]; state.selectedID = draft.id; state.libraryVisible = false
        } else { state.libraryVisible = true; state.sidebarMode = .documents }
        pending.append(state); openWindowAction?()
    }
    func createWindow(in folder: URL) {
        var state = WindowState(); state.currentFolder = folder.path; state.sidebarMode = .folders
        state.libraryVisible = true
        pending.append(state); openWindowAction?()
    }
    func reopenApplication() {
        if let model = active { focus(model) }
        else if pending.isEmpty { createWindow(newDraft: false) }
    }
    func open(_ urls: [URL]) {
        guard let model = active else {
            pendingURLs += urls
            if restored && pending.isEmpty { createWindow(newDraft: false) }
            return
        }
        for url in urls { model.open(url) }
    }
    private func snapshot(from model: AppModel? = nil, excluding: Set<UUID> = [],
                          overriding: [UUID: WindowState] = [:], queued: [WindowState]? = nil,
                          history: [ClosedWorkspaceItem]? = nil) -> SessionState {
        var durable = model?.session ?? library.preferences
        durable.closedItems = history ?? closedItems
        let valid = Set(library.drafts.filter { $0.shouldPersist && $0.deletedAt == nil }.map(\.id))
        let excluded = closingWindowIDs.union(excluding)
        let states = models.filter { !excluded.contains($0.windowID) }.map { overriding[$0.windowID] ?? WindowState(id: $0.windowID, session: $0.session) }
        durable.windows = WindowLayout(windows: states + (queued ?? pending), validIDs: valid).windows
        let primary = durable.windows?.first { $0.id == model?.windowID } ?? durable.windows?.first
        durable.tabs = primary?.tabs ?? []; durable.selectedID = primary?.selectedID
        return durable
    }
    @discardableResult private func save(_ state: SessionState, drafts: [Draft] = [], reportingTo model: AppModel? = nil) -> Bool {
        do {
            guard let store = library.store else { throw TextMDError.storage("Biblioteca indisponível") }
            try store.saveClosing(drafts, session: state)
            library.preferences = state; library.sessionWriteFailed = false
            return true
        } catch {
            library.sessionWriteFailed = true
            if let model { model.report(error) } else { library.errorMessage = error.localizedDescription }
            return false
        }
    }
    func persist(from model: AppModel) {
        guard !closingWindowIDs.contains(model.windowID), models.contains(where: { $0 === model }) else { return }
        for other in models where other !== model {
            other.session.appearance = model.session.appearance
            other.session.metrics = model.session.metrics
            other.session.ignoreMarkdown = model.session.ignoreMarkdown
            other.session.tokenProfile = model.session.tokenProfile
            other.session.folders = model.session.folders
            other.session.zoom = model.session.zoom
            other.refreshMeasurements()
        }
        _ = save(snapshot(from: model), reportingTo: model)
        model.syncWindowTitle()
    }
    func flushAll() {
        for model in models where !closingWindowIDs.contains(model.windowID) { model.flushEditor() }
        _ = save(snapshot(from: active), reportingTo: active)
    }
    private func mayClose(_ model: AppModel) -> Bool {
        model.flushEditor()
        guard library.uncommitted.isEmpty, !library.sessionWriteFailed else {
            model.errorMessage = "Algumas edições ainda não foram guardadas. Tente guardar novamente antes de fechar."
            return false
        }
        return true
    }
    private func historyAdding(_ state: WindowState, kind: ClosedWorkspaceItem.Kind, tabIndex: Int? = nil) -> [ClosedWorkspaceItem] {
        let valid = Set(library.drafts.filter { $0.shouldPersist && $0.deletedAt == nil }.map(\.id))
        var state = state; state.tabs.removeAll { !valid.contains($0) }
        guard !state.tabs.isEmpty else { return closedItems }
        if !state.tabs.contains(state.selectedID ?? UUID()) { state.selectedID = state.tabs.first }
        if kind == .tab { state.libraryVisible = false }
        return Array((closedItems + [ClosedWorkspaceItem(kind: kind, window: state, tabIndex: tabIndex)]).suffix(100))
    }
    func reopenClosed() { if let item = closedItems.last { restoreClosed(item.id) } }
    func restoreLastWindow() { if let item = lastClosedWindow { restoreClosed(item.id) } }
    func restoreClosed(_ id: UUID) {
        guard let item = closedItems.first(where: { $0.id == id }) else { return }
        let history = closedItems.filter { $0.id != id }
        let valid = Set(library.drafts.filter { $0.shouldPersist && $0.deletedAt == nil }.map(\.id))
        var state = item.window
        state.tabs = item.window.tabs.filter { draftID in valid.contains(draftID) && placement(of: draftID) == nil && !pending.contains { $0.tabs.contains(draftID) } }
        guard !state.tabs.isEmpty else {
            if save(snapshot(history: history), reportingTo: active), let owned = item.window.tabs.compactMap({ owner(of: $0) }).first { focus(owned) }
            return
        }
        if !state.tabs.contains(state.selectedID ?? UUID()) { state.selectedID = state.tabs.first }
        if item.kind == .tab { state.libraryVisible = false }
        let target: AppModel?
        if item.kind == .tab { target = models.first { $0.windowID == item.window.id && !closingWindowIDs.contains($0.windowID) } ?? active }
        else { target = active.flatMap { $0.session.tabs.isEmpty ? $0 : nil } }
        if let target {
            target.flushEditor()
            var proposed = item.kind == .window ? state.applying(to: target.session) : target.session
            if item.kind == .tab {
                let position = min(item.tabIndex ?? proposed.tabs.count, proposed.tabs.count)
                proposed.tabs.insert(contentsOf: state.tabs, at: position); proposed.selectedID = state.selectedID
            }
            if item.kind == .tab { proposed.libraryVisible = false }
            let replacement = WindowState(id: target.windowID, session: proposed)
            guard save(snapshot(from: target, overriding: [target.windowID: replacement], history: history), reportingTo: target) else { return }
            target.session = proposed; target.showingTrash = false
            if item.kind == .tab { target.followActiveFolder() }
            target.refreshMeasurements(); target.refreshDiffOriginal(); target.syncWindowTitle()
            focus(target)
        } else {
            if models.contains(where: { $0.windowID == state.id }) || pending.contains(where: { $0.id == state.id }) { state.id = UUID() }
            let queue = pending + [state]
            guard save(snapshot(queued: queue, history: history), reportingTo: active) else { return }
            pending = queue; focusPending.insert(state.id); openWindowAction?()
        }
    }
    func moveTab(_ id: UUID, to target: AppModel, before: UUID? = nil) {
        guard let source = owner(of: id) else { return }
        source.flushEditor(); target.flushEditor()
        guard library.uncommitted.isEmpty, !library.sessionWriteFailed else { _ = mayClose(source); return }
        var layout = WindowLayout(windows: models.filter { !closingWindowIDs.contains($0.windowID) }.map { WindowState(id: $0.windowID, session: $0.session) })
        guard layout.move(id, to: target.windowID, before: before) else { return }
        for index in layout.windows.indices { layout.windows[index].libraryVisible = layout.windows[index].tabs.isEmpty }
        let overrides = Dictionary(uniqueKeysWithValues: layout.windows.map { ($0.id, $0) })
        let emptySource = source !== target && overrides[source.windowID]?.tabs.isEmpty == true
        guard save(snapshot(from: target, excluding: emptySource ? [source.windowID] : [], overriding: overrides), reportingTo: target) else { return }
        for state in layout.windows {
            if let model = models.first(where: { $0.windowID == state.id }) {
                model.session = state.applying(to: model.session); model.session.libraryVisible = model.session.tabs.isEmpty
                model.followActiveFolder(); model.refreshMeasurements(); model.refreshDiffOriginal(); model.syncWindowTitle()
            }
        }
        target.session.libraryVisible = false; focus(target)
        if emptySource { closingWindowIDs.insert(source.windowID); source.nativeWindow?.performClose(nil) }
    }
    func detachTab(_ id: UUID) {
        guard let source = owner(of: id), mayClose(source) else { return }
        let oldIndex = source.session.tabs.firstIndex(of: id) ?? 0
        var proposed = source.session; proposed.tabs.removeAll { $0 == id }
        if proposed.selectedID == id { proposed.selectedID = proposed.tabs.isEmpty ? nil : proposed.tabs[min(oldIndex, proposed.tabs.count - 1)] }
        var state = WindowState(session: source.session); state.tabs = [id]; state.selectedID = id; state.libraryVisible = false
        let queue = pending + [state], empty = proposed.tabs.isEmpty
        guard save(snapshot(from: source, excluding: empty ? [source.windowID] : [], overriding: [source.windowID: WindowState(id: source.windowID, session: proposed)], queued: queue), reportingTo: source) else { return }
        source.session = proposed; pending = queue; focusPending.insert(state.id)
        source.followActiveFolder(); source.refreshMeasurements(); source.refreshDiffOriginal(); source.syncWindowTitle()
        openWindowAction?()
        if empty { closingWindowIDs.insert(source.windowID); source.nativeWindow?.performClose(nil) }
    }
    @discardableResult func closeTab(_ id: UUID, in model: AppModel) -> Bool {
        guard model.session.tabs.contains(id), mayClose(model), var draft = library.drafts.first(where: { $0.id == id }) else { return false }
        let index = model.session.tabs.firstIndex(of: id)!
        var proposed = model.session; proposed.tabs.removeAll { $0 == id }
        if proposed.selectedID == id { proposed.selectedID = proposed.tabs.isEmpty ? nil : proposed.tabs[min(index, proposed.tabs.count - 1)] }
        var closedState = WindowState(id: model.windowID, session: model.session)
        closedState.tabs = [id]; closedState.selectedID = id
        let empty = proposed.tabs.isEmpty
        let history = historyAdding(closedState, kind: empty ? .window : .tab, tabIndex: index)
        if draft.deletedAt == nil { draft.hiddenFromSidebar = false }
        let durable = snapshot(from: model, excluding: empty ? [model.windowID] : [], overriding: [model.windowID: WindowState(id: model.windowID, session: proposed)], history: history)
        guard save(durable, drafts: draft.shouldPersist ? [draft] : [], reportingTo: model) else { return false }
        if let i = library.drafts.firstIndex(where: { $0.id == id }) {
            if draft.shouldPersist { library.drafts[i] = draft } else { library.drafts.remove(at: i) }
        }
        model.session = proposed; library.discardEditor(id)
        model.followActiveFolder(); model.refreshMeasurements(); model.refreshDiffOriginal(); model.syncWindowTitle()
        if empty { closingWindowIDs.insert(model.windowID); model.nativeWindow?.performClose(nil) }
        return true
    }
    func prepareToClose(_ model: AppModel) -> Bool {
        if terminating || closingWindowIDs.contains(model.windowID) { return true }
        guard mayClose(model) else { return false }
        let state = WindowState(id: model.windowID, session: model.session)
        let history = historyAdding(state, kind: .window)
        guard save(snapshot(excluding: [model.windowID], history: history), reportingTo: model) else { return false }
        closingWindowIDs.insert(model.windowID)
        return true
    }
    /// Called only after a successful close. Testable without creating NSWindow.
    func completeClose(_ model: AppModel) {
        guard closingWindowIDs.contains(model.windowID), !terminating else { return }
        for id in model.session.tabs { library.discardEditor(id) }
        model.stopObserving(); models.removeAll { $0 === model }; model.coordinator = nil
        closingWindowIDs.remove(model.windowID)
        if activeWindowID == model.windowID { activeWindowID = models.last?.windowID }
    }
    func windowDidBecomeKey(_ notification: Notification) {
        if let window = notification.object as? NSWindow { activeWindowID = models.first { $0.nativeWindow === window }?.windowID }
    }
    func windowShouldClose(_ sender: NSWindow) -> Bool {
        guard let model = models.first(where: { $0.nativeWindow === sender }) else { return true }
        return prepareToClose(model)
    }
    func windowWillClose(_ notification: Notification) {
        guard let window = notification.object as? NSWindow, let model = models.first(where: { $0.nativeWindow === window }) else { return }
        completeClose(model)
    }
}
