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
    @ObservationIgnored var store: DraftStore?
    static func rootURL() throws -> URL {
        if let value = ProcessInfo.processInfo.environment["TEXTMD_LIBRARY_PATH"] { return URL(fileURLWithPath: value, isDirectory: true) }
        return try FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask,
            appropriateFor: nil, create: true).appendingPathComponent("text-md", isDirectory: true)
    }
    init() {
        do {
            let store = try DraftStore(root: Self.rootURL()); self.store = store
            try store.cleanup(); try store.recoverLegacyEmptyDrafts()
            preferences = try store.session()
            try store.hideLegacyClosedDrafts(session: preferences)
            drafts = try store.allDrafts()
        } catch { self.store = nil; errorMessage = error.localizedDescription }
    }
}

@MainActor @Observable
final class WindowCoordinator: NSObject, NSWindowDelegate {
    static let shared = WindowCoordinator()
    let library = AppLibrary()
    private(set) var models: [AppModel] = []
    var activeWindowID: UUID?
    @ObservationIgnored var openWindowAction: (() -> Void)?
    @ObservationIgnored private var pending: [WindowState] = []
    @ObservationIgnored private var closed: [(UUID, UUID)] = []
    @ObservationIgnored private var restored = false
    @ObservationIgnored var terminating = false
    @ObservationIgnored private var pendingURLs: [URL] = []
    @ObservationIgnored private var pendingPanel: Bool?

    override init() {
        super.init()
        let session = library.preferences
        let candidates = session.windows ?? [WindowState(session: session)]
        let valid = Set(library.drafts.filter { $0.deletedAt == nil }.map(\.id))
        let identities = Dictionary(uniqueKeysWithValues: library.drafts.compactMap { draft -> (UUID, String)? in
            draft.sourcePath.map { (draft.id, FileService.documentKey(URL(fileURLWithPath: $0))) }
        })
        pending = WindowLayout(windows: candidates, validIDs: valid, identities: identities).windows
    }
    var active: AppModel? { models.first { $0.nativeWindow === NSApp.keyWindow } ?? models.first { $0.windowID == activeWindowID } ?? models.last }
    func claimWindow() -> AppModel {
        let state = pending.isEmpty ? WindowState() : pending.removeFirst()
        let model = AppModel(library: library, state: state); model.coordinator = self
        models.append(model)
        if model.session.tabs.isEmpty { model.newDraft() }
        return model
    }
    func attach(_ model: AppModel, to window: NSWindow) {
        model.nativeWindow = window; window.delegate = self
        window.identifier = NSUserInterfaceItemIdentifier(model.windowID.uuidString)
        window.setAccessibilityIdentifier(model.windowID.uuidString)
        window.tabbingMode = .disallowed
        if window.isKeyWindow || activeWindowID == nil { activeWindowID = model.windowID }
        model.checkDisk()
        if !restored {
            restored = true
            let count = pending.count
            for _ in 0..<count { openWindowAction?() }
        }
        if !pendingURLs.isEmpty {
            let urls = pendingURLs; pendingURLs = []
            for url in urls { model.open(url) }
        }
        persist(from: model)
        if let folder = pendingPanel {
            pendingPanel = nil
            DispatchQueue.main.async { if folder { model.openFolderPanel() } else { model.openPanel() } }
        }
    }
    func openPanel(folder: Bool) {
        if let model = active { if folder { model.openFolderPanel() } else { model.openPanel() } }
        else { pendingPanel = folder; createWindow() }
    }
    func placement(of id: UUID) -> (AppModel, UUID)? {
        if let model = models.first(where: { $0.session.tabs.contains(id) }) { return (model, id) }
        guard let path = library.drafts.first(where: { $0.id == id })?.sourcePath else { return nil }
        let key = FileService.documentKey(URL(fileURLWithPath: path))
        for model in models {
            for owned in model.session.tabs {
                if let candidate = library.drafts.first(where: { $0.id == owned })?.sourcePath,
                   FileService.documentKey(URL(fileURLWithPath: candidate)) == key { return (model, owned) }
            }
        }
        return nil
    }
    func owner(of id: UUID) -> AppModel? { placement(of: id)?.0 }
    func createWindow() { pending.append(WindowState()); openWindowAction?() }
    func createWindow(in folder: URL) {
        var state = WindowState(); state.currentFolder = folder.path; state.sidebarMode = .folders
        pending.append(state); openWindowAction?()
    }
    func reopenApplication() {
        if let model = active, let window = model.nativeWindow {
            window.deminiaturize(nil); window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
        } else { createWindow() }
    }
    func open(_ urls: [URL]) {
        guard let model = active else { pendingURLs += urls; if restored { createWindow() }; return }
        for url in urls { model.open(url) }
    }
    func rememberClosed(_ id: UUID, in model: AppModel) {
        guard let draft = library.drafts.first(where: { $0.id == id }), draft.shouldPersist, draft.deletedAt == nil else { return }
        closed.append((id, model.windowID))
    }
    func reopenClosed() {
        while let (id, windowID) = closed.popLast() {
            guard library.drafts.contains(where: { $0.id == id && $0.deletedAt == nil }) else { continue }
            if let model = owner(of: id) ?? models.first(where: { $0.windowID == windowID }) ?? active {
                model.select(id); model.nativeWindow?.makeKeyAndOrderFront(nil)
            } else {
                var session = library.preferences; session.tabs = [id]; session.selectedID = id
                pending.append(WindowState(session: session)); openWindowAction?()
            }
            return
        }
    }
    func moveTab(_ id: UUID, to target: AppModel, before: UUID? = nil) {
        guard let source = owner(of: id) else { return }
        source.flushEditor(); target.flushEditor()
        var layout = WindowLayout(windows: models.map { WindowState(id: $0.windowID, session: $0.session) })
        guard layout.move(id, to: target.windowID, before: before) else { return }
        for state in layout.windows {
            guard let model = models.first(where: { $0.windowID == state.id }) else { continue }
            model.session.tabs = state.tabs; model.session.selectedID = state.selectedID
            model.followActiveFolder(); model.refreshMeasurements(); model.refreshDiffOriginal()
        }
        target.select(id); target.nativeWindow?.makeKeyAndOrderFront(nil)
        persist(from: target)
        if source !== target && source.session.tabs.isEmpty { source.nativeWindow?.performClose(nil) }
    }
    func detachTab(_ id: UUID) {
        guard let source = owner(of: id) else { return }
        source.flushEditor()
        var session = source.session; session.tabs = [id]; session.selectedID = id
        pending.append(WindowState(session: session))
        source.session.tabs.removeAll { $0 == id }
        if source.session.selectedID == id { source.session.selectedID = source.session.tabs.last; source.followActiveFolder() }
        persist(from: source); openWindowAction?()
        source.refreshMeasurements(); source.refreshDiffOriginal()
        if source.session.tabs.isEmpty { source.nativeWindow?.performClose(nil) }
    }
    func persist(from model: AppModel) {
        // Broadcast only app-wide preferences. Navigation and editor modes stay local.
        for other in models where other !== model {
            other.session.appearance = model.session.appearance
            other.session.metrics = model.session.metrics
            other.session.ignoreMarkdown = model.session.ignoreMarkdown
            other.session.tokenProfile = model.session.tokenProfile
            other.session.folders = model.session.folders
            other.session.zoom = model.session.zoom
            other.refreshMeasurements()
        }
        var durable = model.session
        let valid = Set(library.drafts.filter { $0.shouldPersist && $0.deletedAt == nil }.map(\.id))
        durable.windows = WindowLayout(windows: models.map { WindowState(id: $0.windowID, session: $0.session) } + pending, validIDs: valid).windows
        durable.tabs = durable.windows?.first(where: { $0.id == model.windowID })?.tabs ?? []
        durable.selectedID = durable.windows?.first(where: { $0.id == model.windowID })?.selectedID
        library.preferences = durable
        do {
            guard let store = library.store else { throw TextMDError.storage("Biblioteca indisponível") }
            try store.saveSession(durable); library.sessionWriteFailed = false
        } catch { library.sessionWriteFailed = true; model.report(error) }
    }
    func flushAll() { for model in models { model.flushEditor() }; if let model = active { persist(from: model) } }
    func windowDidBecomeKey(_ notification: Notification) {
        if let window = notification.object as? NSWindow { activeWindowID = models.first { $0.nativeWindow === window }?.windowID }
    }
    func windowShouldClose(_ sender: NSWindow) -> Bool {
        let model = models.first { $0.nativeWindow === sender }; model?.flushEditor()
        if !terminating, let model { for id in model.session.tabs { model.hideFromSidebar(id) } }
        if library.uncommitted.isEmpty && !library.sessionWriteFailed {
            var snapshot = library.preferences
            let valid = Set(library.drafts.filter { $0.shouldPersist && $0.deletedAt == nil }.map(\.id))
            snapshot.windows = WindowLayout(windows: models.filter { $0.nativeWindow !== sender }.map { WindowState(id: $0.windowID, session: $0.session) } + pending, validIDs: valid).windows
            snapshot.tabs = snapshot.windows?.first?.tabs ?? []; snapshot.selectedID = snapshot.windows?.first?.selectedID
            do { try library.store?.saveSession(snapshot); return true }
            catch { library.sessionWriteFailed = true; model?.report(error); return false }
        }
        model?.errorMessage = "Algumas edições ainda não foram guardadas. Mantenha a janela aberta e tente guardar novamente ou salve uma cópia."
        return false
    }
    func windowWillClose(_ notification: Notification) {
        guard !terminating, let window = notification.object as? NSWindow,
              let model = models.first(where: { $0.nativeWindow === window }) else { return }
        model.flushEditor()
        for id in model.session.tabs { rememberClosed(id, in: model); model.hideFromSidebar(id) }
        model.stopObserving(); models.removeAll { $0 === model }
        if activeWindowID == model.windowID { activeWindowID = models.last?.windowID }
        if let remaining = active { persist(from: remaining) }
        else {
            var session = library.preferences; session.windows = pending; session.tabs = []; session.selectedID = nil
            library.preferences = session
            do { try library.store?.saveSession(session) } catch { library.errorMessage = error.localizedDescription }
        }
    }
}
