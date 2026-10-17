import AppKit
import Observation
import TextMDCore
import UniformTypeIdentifiers

struct FileNode: Identifiable {
    let url: URL
    let isDirectory: Bool
    var id: String { url.path }
    var name: String { url.lastPathComponent }
    var isEditable: Bool { FileService.isEditable(url) }
}
struct ConflictPresentation: Identifiable {
    let id: UUID
    let disk: DiskState
}
struct HistoryPresentation: Identifiable { let id: UUID }

@MainActor @Observable
final class AppModel {
    let library: AppLibrary
    let windowID: UUID
    @ObservationIgnored weak var coordinator: WindowCoordinator?
    @ObservationIgnored weak var nativeWindow: NSWindow?
    private(set) var drafts: [Draft] {
        get { library.drafts }
        set { library.drafts = newValue }
    }
    var session = SessionState()
    var search = ""
    var fileResults: [FileSearchResult] = []
    var searchRunning = false
    var searchWarning: String?
    var showingTrash = false
    var showClosedDrafts = false
    var sidebarVisible: Bool {
        get { session.sidebarVisible ?? true }
        set { session.sidebarVisible = newValue }
    }
    var searchVisible = false
    var searchFocusRequest = 0
    var pendingAnchor: (id: UUID, name: String)?
    @ObservationIgnored private var sidebarSaveWork: DispatchWorkItem?
    var sidebarWidth: Double { min(350, max(190, session.sidebarWidth ?? 245)) }
    var showingLibrary: Bool { session.libraryVisible ?? (active == nil) }
    var hasDocumentContext: Bool { !showingTrash && !showingLibrary && active != nil }
    func showLibrary() {
        flushEditor(); showingTrash = false; session.libraryVisible = true
        session.sidebarMode = .documents; session.sidebarVisible = true
        persistSession(); syncWindowTitle()
    }
    func syncWindowTitle() {
        nativeWindow?.title = showingTrash ? "Lixeira — text-md" : showingLibrary ? "Biblioteca — text-md" : (active?.title ?? "text-md") + " — text-md"
        nativeWindow?.representedURL = hasDocumentContext ? active?.sourcePath.map { URL(fileURLWithPath: $0) } : nil
    }
    func focusSearch() {
        sidebarVisible = true; searchVisible = true; searchFocusRequest += 1
        persistSession()
    }
    func setSidebarWidth(_ value: Double) {
        let value = min(350, max(190, value))
        guard abs(value - sidebarWidth) > 0.5 else { return }
        session.sidebarWidth = value
        sidebarSaveWork?.cancel()
        let work = DispatchWorkItem { [weak self] in self?.persistSession() }
        sidebarSaveWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2, execute: work)
    }
    func enterTrash() { flushEditor(); showingTrash = true; syncWindowTitle() }
    func leaveTrash() {
        showingTrash = false
        if active == nil { session.libraryVisible = true }; syncWindowTitle()
    }
    func findInDocument() { guard hasDocumentContext else { return }; editor?.showFind() }
    var errorMessage: String?
    var conflict: ConflictPresentation?
    var history: HistoryPresentation?
    var diskStates: [UUID: DiskState] = [:]
    var uncommitted: Set<UUID> {
        get { library.uncommitted }
        set { library.uncommitted = newValue }
    }
    var status = "Rascunhos guardados neste Mac"
    @ObservationIgnored weak var editor: MarkdownTextView?
    var children: [String: [FileNode]] = [:]
    var folderErrors: [String: String] = [:]
    var counts = DocumentCounts(markdown: "", ignoreMarkdown: true)
    var tokenCount: Int?
    var tokenError: String?
    var tokensPending = false
    var originalForDiff: String?
    var diffWarning: String?
    var store: DraftStore? { library.store }
    @ObservationIgnored private var pollTimer: Timer?
    @ObservationIgnored private var searchTask: Task<Void, Never>?
    @ObservationIgnored private var alertFingerprint: [UUID: String] = [:]
    private var undoManagers: [UUID: UndoManager] {
        get { library.undoManagers }
        set { library.undoManagers = newValue }
    }
    @ObservationIgnored private var lastCleanup = Date()
    @ObservationIgnored private var tokenTask: Task<Void, Never>?
    @ObservationIgnored private var countedText: String?
    @ObservationIgnored private var countedProfile: TokenProfile?

    init(library: AppLibrary, state: WindowState) {
        self.library = library; windowID = state.id
        session = state.applying(to: library.preferences)
        if let error = library.errorMessage { errorMessage = error; status = "Biblioteca indisponível — edição desativada" }
        refreshExpandedFolders(); refreshMeasurements()
        pollTimer = Timer.scheduledTimer(withTimeInterval: 2, repeats: true) { [weak self] _ in
            Task { @MainActor in self?.checkDisk(); self?.refreshExpandedFolders() }
        }
    }
    func stopObserving() { pollTimer?.invalidate(); searchTask?.cancel(); tokenTask?.cancel(); sidebarSaveWork?.cancel() }
    var zoom: CGFloat { CGFloat(min(3, max(0.5, session.zoom ?? 1))) }
    var sidebarMode: SidebarMode { session.sidebarMode ?? .documents }
    var currentFolder: String? { session.currentFolder }
    func toggleSection(_ key: String) {
        var values = session.collapsedSections ?? []
        if values.contains(key) { values.removeAll { $0 == key } } else { values.append(key) }
        session.collapsedSections = values; persistSession()
    }
    func sectionExpanded(_ key: String) -> Bool { !(session.collapsedSections ?? []).contains(key) }
    func setSidebarMode(_ mode: SidebarMode) {
        session.sidebarMode = mode
        if mode == .folders, session.currentFolder == nil, let folder = session.folders.first { session.currentFolder = folder.path }
        refreshExpandedFolders(); persistSession()
    }
    func navigateFolder(_ url: URL) {
        let path = url.standardizedFileURL.path
        session.currentFolder = path; session.sidebarMode = .folders
        session.collapsedSections?.removeAll { $0 == "folders" }
        loadChildren(path); persistSession()
    }
    func openFolderPanel() {
        let panel = NSOpenPanel(); panel.canChooseFiles = false; panel.canChooseDirectories = true
        if panel.runModal() == .OK, let url = panel.url { navigateFolder(url) }
    }
    func togglePinnedFolder(_ url: URL) {
        let key = FileService.documentKey(url)
        if let folder = session.folders.first(where: { FileService.documentKey(URL(fileURLWithPath: $0.path)) == key }) { unpinFolder(folder.id) }
        else {
            var folder = PinnedFolder(path: url.path)
            folder.bookmark = try? url.bookmarkData(options: [], includingResourceValuesForKeys: nil, relativeTo: nil)
            session.folders.append(folder); persistSession()
        }
    }
    func folderIsPinned(_ url: URL) -> Bool {
        session.folders.contains { FileService.documentKey(URL(fileURLWithPath: $0.path)) == FileService.documentKey(url) }
    }
    func setZoom(_ value: Double) { session.zoom = min(3, max(0.5, value)); persistSession() }
    var active: Draft? { drafts.first { $0.id == session.selectedID } }
    var visibleDrafts: [Draft] {
        let openIDs = Set(coordinator?.models.flatMap { $0.session.tabs } ?? session.tabs)
        return drafts.filter { $0.visibleInSidebar(openIDs: openIDs) && matches($0) }.sorted {
            if $0.pinned != $1.pinned { return $0.pinned }
            return $0.modifiedAt > $1.modifiedAt
        }
    }
    var hiddenDrafts: [Draft] {
        let openIDs = Set(coordinator?.models.flatMap { $0.session.tabs } ?? session.tabs)
        return drafts.filter { $0.deletedAt == nil && $0.shouldPersist && !openIDs.contains($0.id) }.sorted { $0.modifiedAt > $1.modifiedAt }
    }
    var trash: [Draft] { drafts.filter { $0.deletedAt != nil && matches($0) }.sorted { ($0.deletedAt ?? .distantPast) > ($1.deletedAt ?? .distantPast) } }
    var tabs: [Draft] { session.tabs.compactMap { id in drafts.first { $0.id == id } } }
    var hasUnsavedMemory: Bool { !uncommitted.isEmpty || library.sessionWriteFailed }
    var diffEnabled: Bool { session.diffEnabled == true }
    var tokenProfile: TokenProfile { session.tokenProfile ?? .o200k }
    var enabledMetrics: [DocumentMetric] { session.metrics ?? [.words, .tokens] }
    private func matches(_ draft: Draft) -> Bool {
        search.isEmpty || draft.title.localizedStandardContains(search) || draft.content.localizedStandardContains(search)
    }
    func report(_ error: Error) { errorMessage = error.localizedDescription }
    private func requireStore() throws -> DraftStore {
        guard let store else { throw TextMDError.storage("A biblioteca não está disponível.") }
        return store
    }
    func persistSession() { coordinator?.persist(from: self) }
    @discardableResult private func commit(_ draft: Draft, snapshot: Revision? = nil) -> Bool {
        let previousPath = drafts.first { $0.id == draft.id }?.sourcePath
        if let index = drafts.firstIndex(where: { $0.id == draft.id }) { drafts[index] = draft }
        else { drafts.insert(draft, at: 0) }
        for model in coordinator?.models ?? [self] where draft.id == model.session.selectedID { model.refreshMeasurements() }
        guard draft.shouldPersist else {
            status = "Escreva para guardar este rascunho"
            return true
        }
        var snapshots = library.pendingSnapshots[draft.id] ?? []
        if let snapshot, !snapshots.contains(where: { $0.id == snapshot.id }) { snapshots.append(snapshot) }
        do {
            try requireStore().save(draft, snapshots: snapshots)
            library.pendingSnapshots.removeValue(forKey: draft.id)
            uncommitted.remove(draft.id)
            status = "Rascunho guardado"
            if let editor = library.editorScrollViews[draft.id]?.documentView as? MarkdownTextView,
               editor.string != draft.content, let owner = editor.model {
                editor.configure(draft: draft, source: owner.session.sourceMode, lineNumbers: owner.session.showLineNumbers,
                                 diff: owner.diffEnabled, original: owner.originalForDiff, zoom: owner.zoom)
            }
            for model in coordinator?.models ?? [self] {
                if draft.sourcePath != previousPath, model.active?.id == draft.id { model.followActiveFolder() }
                model.syncWindowTitle()
            }
            return true
        } catch {
            library.pendingSnapshots[draft.id] = snapshots
            uncommitted.insert(draft.id)
            status = "Falha ao guardar — mantenha o app aberto"
            report(error); return false
        }
    }
    func update(id: UUID, content: String, selection: Int, scroll: Double) {
        guard var draft = drafts.first(where: { $0.id == id }) else { return }
        let changed = draft.content != content
        let old = draft
        draft.content = content; draft.selection = selection; draft.scrollPosition = scroll
        if !content.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty { draft.hasWrittenContent = true }
        if changed { draft.modifiedAt = Date() }
        let snapshot: Revision?
        if changed && !old.content.isEmpty && (old.lastSnapshotAt == nil || Date().timeIntervalSince(old.lastSnapshotAt!) >= 30) {
            snapshot = Revision(draftID: id, content: old.content, reason: "Durante a edição")
            draft.lastSnapshotAt = Date()
        } else { snapshot = nil }
        _ = commit(draft, snapshot: snapshot)
        // The first real edit makes this formerly temporary tab restorable.
        if !old.shouldPersist && draft.shouldPersist { persistSession() }
    }
    func flushEditor() { editor?.flush() }
    func newDraft(content: String = "") {
        flushEditor()
        let draft = Draft(content: content)
        guard commit(draft) else { return }
        select(draft.id)
    }
    func select(_ id: UUID, newTab: Bool = true) {
        flushEditor()
        if coordinator?.routePendingDocument(id) == true { return }
        if let (owner, placedID) = coordinator?.placement(of: id), owner !== self || placedID != id {
            flushEditor(); owner.select(placedID); owner.nativeWindow?.deminiaturize(nil)
            owner.nativeWindow?.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true); return
        }
        flushEditor()
        showingTrash = false; session.libraryVisible = false
        if var draft = drafts.first(where: { $0.id == id }), draft.hiddenFromSidebar != false {
            draft.hiddenFromSidebar = false; _ = commit(draft)
        }
        if !session.tabs.contains(id) {
            if !newTab, let old = session.selectedID, let index = session.tabs.firstIndex(of: old) {
                guard preserveDraftForReplacement(old) else { return }
                session.tabs[index] = id
                if let draft = drafts.first(where: { $0.id == old }), !draft.shouldPersist { drafts.removeAll { $0.id == old }; undoManagers.removeValue(forKey: old) }
            } else { session.tabs.append(id) }
        }
        session.selectedID = id
        followActiveFolder()
        persistSession()
        checkDisk()
        refreshMeasurements(); refreshDiffOriginal()
    }
    func followActiveFolder() {
        guard let path = active?.sourcePath else { return }
        session.currentFolder = URL(fileURLWithPath: path).deletingLastPathComponent().path
        session.sidebarMode = .folders; session.collapsedSections?.removeAll { $0 == "folders" }
        loadChildren(session.currentFolder!)
    }
    func closeTab(_ id: UUID) {
        guard let coordinator else { return }
        coordinator.closeTab(id, in: self)
    }
    func switchTab(_ delta: Int) {
        guard !session.tabs.isEmpty else { return }
        let index = session.tabs.firstIndex(of: session.selectedID ?? UUID()) ?? 0
        select(session.tabs[(index + delta + session.tabs.count) % session.tabs.count])
    }
    func selectTab(_ number: Int) {
        let index = number == 9 ? session.tabs.count - 1 : number - 1
        if session.tabs.indices.contains(index) { select(session.tabs[index]) }
    }
    func toggleSource() { guard hasDocumentContext else { return }; flushEditor(); session.sourceMode.toggle(); persistSession() }
    func toggleLines() { session.showLineNumbers.toggle(); persistSession() }
    func toggleDiff() { guard hasDocumentContext else { return }; flushEditor(); session.diffEnabled = !diffEnabled; refreshDiffOriginal(); persistSession() }
    func setAppearance(_ value: EditorAppearance) { session.appearance = value; persistSession() }
    func setMetric(_ value: DocumentMetric, enabled: Bool) {
        var values = enabledMetrics.filter { $0 != value }
        if enabled { values.append(value) }
        session.metrics = DocumentMetric.allCases.filter { values.contains($0) }; persistSession()
    }
    func setIgnoreMarkdown(_ value: Bool) { session.ignoreMarkdown = value; refreshMeasurements(); persistSession() }
    func setTokenProfile(_ value: TokenProfile) { session.tokenProfile = value; refreshMeasurements(); persistSession() }

    func refreshMeasurements() {
        let text = active?.content ?? "", profile = tokenProfile
        counts = DocumentCounts(markdown: text, ignoreMarkdown: session.ignoreMarkdown ?? true)
        guard countedText != text || countedProfile != profile else { return }
        countedText = text; countedProfile = profile
        tokenTask?.cancel(); tokenError = nil; tokenCount = nil
        if text.isEmpty { tokensPending = false; tokenCount = 0; return }
        tokensPending = true
        tokenTask = Task { [weak self] in
            do {
                try await Task.sleep(for: .milliseconds(120))
                let tokenizer = try await TokenizerRegistry.shared.tokenizer(profile)
                try Task.checkCancellation()
                let worker = Task.detached(priority: .userInitiated) { try tokenizer.count(text) }
                let count = try await withTaskCancellationHandler(operation: { try await worker.value }, onCancel: { worker.cancel() })
                guard !Task.isCancelled else { return }
                self?.tokenCount = count; self?.tokensPending = false
            } catch is CancellationError {} catch {
                guard !Task.isCancelled else { return }
                self?.tokenError = error.localizedDescription; self?.tokensPending = false
            }
        }
    }
    func refreshDiffOriginal() {
        guard diffEnabled, let draft = active else { originalForDiff = nil; diffWarning = nil; return }
        guard let path = draft.sourcePath else {
            if originalForDiff != "" { originalForDiff = "" }; diffWarning = nil; return
        }
        do {
            let value = try FileService.open(URL(fileURLWithPath: path)).content
            if originalForDiff != value { originalForDiff = value }
            diffWarning = nil
        } catch {
            originalForDiff = nil; diffWarning = "Diff indisponível: " + error.localizedDescription
        }
    }

    func openPanel() {
        let panel = NSOpenPanel(); panel.allowedContentTypes = [.plainText, .init(filenameExtension: "md") ?? .plainText, .init(filenameExtension: "markdown") ?? .plainText]
        panel.allowsMultipleSelection = true; panel.canChooseDirectories = false
        panel.message = "Abra um arquivo para editar seu rascunho. O original só muda ao salvar."
        if panel.runModal() == .OK { for url in panel.urls { open(url) } }
    }
    func open(_ url: URL, newTab: Bool = true) {
        guard FileService.isEditable(url) else {
            if !NSWorkspace.shared.open(url) { errorMessage = "O Mac não conseguiu abrir este arquivo." }
            return
        }
        do {
            let canonical = FileService.documentKey(url)
            if let draft = drafts.first(where: { $0.sourcePath.map { FileService.documentKey(URL(fileURLWithPath: $0)) == canonical } == true && $0.deletedAt == nil }) {
                select(draft.id, newTab: newTab)
                coordinator?.owner(of: draft.id)?.nativeWindow?.makeKeyAndOrderFront(nil)
                return
            }
            let draft = try FileService.open(url)
            if commit(draft) { select(draft.id, newTab: newTab); nativeWindow?.makeKeyAndOrderFront(nil) }
        } catch { report(error) }
    }
    @discardableResult func openMarkdownReference(_ reference: String) -> Bool {
        let components = URLComponents(string: reference)
        if components?.scheme == nil, components?.path.isEmpty == true, let anchor = components?.fragment {
            guard hasDocumentContext else { return false }
            editor?.jumpToHeading(anchor); return true
        }
        guard let url = FileService.linkedMarkdown(reference, from: active?.referencePath) else { return false }
        open(url)
        if let anchor = components?.fragment, let draft = draftForFile(url) {
            let owner = coordinator?.owner(of: draft.id) ?? self
            owner.pendingAnchor = (draft.id, anchor)
            if owner.editor?.draftID == draft.id { owner.editor?.consumeAnchorRequest() }
        }
        return true
    }
    func saveActive(asCopy: Bool = false) {
        guard hasDocumentContext else { return }
        flushEditor()
        guard let draft = active else { return }
        if asCopy || draft.sourcePath == nil { savePanel(draft, asCopy: asCopy); return }
        let disk = FileService.diskState(for: draft)
        guard disk == .unchanged else { conflict = ConflictPresentation(id: draft.id, disk: disk); return }
        write(draft, destination: nil)
    }
    func savePanel(_ draft: Draft, asCopy: Bool) {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.init(filenameExtension: "md") ?? .plainText]
        panel.canCreateDirectories = true
        let name = URL(fileURLWithPath: draft.title).deletingPathExtension().lastPathComponent
        panel.nameFieldStringValue = (asCopy ? name + "-copia" : name) + ".md"
        if let path = draft.sourcePath { panel.directoryURL = URL(fileURLWithPath: path).deletingLastPathComponent() }
        panel.message = asCopy ? "Guarde seu rascunho em outro arquivo. O original será preservado." : "Escolha onde salvar seu arquivo Markdown."
        if panel.runModal() == .OK, let url = panel.url {
            // NSSavePanel handles overwrite confirmation; a copy must never target its conflicted original.
            if asCopy, let path = draft.sourcePath,
               url.resolvingSymlinksInPath().path == URL(fileURLWithPath: path).resolvingSymlinksInPath().path {
                errorMessage = "Escolha outro nome para a cópia. O original está preservado."; return
            }
            write(draft, destination: url, overwriteCopy: true)
        }
    }
    @discardableResult func write(_ draft: Draft, destination: URL?, overwriteCopy: Bool = false) -> Bool {
        do {
            let store = try requireStore()
            try validateSaveDestination(draft, destination: destination)
            // Persist before the filesystem write; failed exports leave a recoverable draft.
            let snapshot = Revision(draftID: draft.id, content: draft.content, reason: "Antes de salvar")
            try store.save(draft, snapshot: snapshot)
            let saved = try FileService.save(draft, to: destination, overwriteCopy: overwriteCopy, assetsRoot: store.assetsRoot)
            guard commit(saved) else { return false }
            diskStates.removeValue(forKey: draft.id); conflict = nil
            alertFingerprint.removeValue(forKey: draft.id)
            status = "Arquivo salvo · rascunho guardado"
            if let path = saved.sourcePath { navigateFolder(URL(fileURLWithPath: path).deletingLastPathComponent()) }
            refreshExpandedFolders()
            refreshDiffOriginal()
            return true
        } catch {
            if case TextMDError.conflict = error { conflict = ConflictPresentation(id: draft.id, disk: FileService.diskState(for: draft)) }
            else { report(error) }
            return false
        }
    }
    func validateSaveDestination(_ draft: Draft, destination: URL?) throws {
        guard let path = destination?.path ?? draft.sourcePath else { return }
        let key = FileService.documentKey(URL(fileURLWithPath: path))
        if drafts.contains(where: { other in
            other.id != draft.id && other.deletedAt == nil && other.sourcePath.map { FileService.documentKey(URL(fileURLWithPath: $0)) == key } == true
        }) {
            throw TextMDError.invalidDestination("Este arquivo já tem um rascunho no text-md. Escolha outro nome para preservar os dois textos.")
        }
    }
    func checkDisk() {
        if !hasUnsavedMemory, Date().timeIntervalSince(lastCleanup) > 3600, let store {
            do {
                let expired = Set(try store.cleanup())
                drafts.removeAll { expired.contains($0.id) }
                session.tabs.removeAll { expired.contains($0) }
                if let id = session.selectedID, expired.contains(id) { session.selectedID = session.tabs.last }
                for id in expired { undoManagers.removeValue(forKey: id) }
                lastCleanup = Date()
            } catch { report(error) }
        }
        for draft in drafts where draft.deletedAt == nil && draft.sourcePath != nil {
            let state = FileService.diskState(for: draft)
            if state == .unchanged { diskStates.removeValue(forKey: draft.id); continue }
            if !draft.isModified, case .changed = state, draft.sourcePath != nil {
                do {
                    let updated = try FileService.reload(draft)
                    if commit(updated, snapshot: Revision(draftID: draft.id, content: draft.content, reason: "Antes da alteração externa")) {
                        diskStates.removeValue(forKey: draft.id)
                    }
                } catch { report(error) }
            } else {
                diskStates[draft.id] = state
                let fingerprint = String(describing: state)
                if draft.id == session.selectedID, conflict == nil, alertFingerprint[draft.id] != fingerprint {
                    alertFingerprint[draft.id] = fingerprint
                    conflict = ConflictPresentation(id: draft.id, disk: state)
                }
            }
        }
        refreshDiffOriginal()
    }
    func discardAndReload(_ id: UUID) {
        guard let draft = drafts.first(where: { $0.id == id }), draft.sourcePath != nil else { return }
        do {
            let current = try FileService.reload(draft)
            if commit(current, snapshot: Revision(draftID: id, content: draft.content, reason: "Antes de descartar o rascunho")) {
                diskStates.removeValue(forKey: id); conflict = nil
            }
        } catch { report(error) }
    }
    func showHistory() { guard hasDocumentContext else { return }; flushEditor(); if let id = active?.id { history = HistoryPresentation(id: id) } }
    func revisions(for id: UUID) -> [Revision] {
        do { return try requireStore().revisions(for: id) } catch { report(error); return [] }
    }
    func restore(_ revision: Revision) {
        guard var draft = drafts.first(where: { $0.id == revision.draftID }) else { return }
        let backup = Revision(draftID: draft.id, content: draft.content, reason: "Antes de restaurar versão")
        draft.content = revision.content; draft.modifiedAt = Date(); draft.deletedAt = nil
        if commit(draft, snapshot: backup) { history = nil; select(draft.id) }
    }
    func pin(_ id: UUID) {
        guard var draft = drafts.first(where: { $0.id == id }) else { return }
        draft.pinned.toggle(); draft.hiddenFromSidebar = false; _ = commit(draft)
    }
    @discardableResult private func preserveDraftForReplacement(_ id: UUID) -> Bool {
        guard var draft = drafts.first(where: { $0.id == id }), draft.shouldPersist else { return true }
        draft.hiddenFromSidebar = false
        return commit(draft)
    }
    func draftForFile(_ url: URL) -> Draft? {
        drafts.first { $0.deletedAt == nil && $0.sourcePath.map { FileService.documentKey(URL(fileURLWithPath: $0)) == FileService.documentKey(url) } == true }
    }
    func toggleFilePin(_ url: URL) {
        do {
            let draft: Draft
            if let existing = draftForFile(url) { draft = existing }
            else { let opened = try FileService.open(url); guard commit(opened) else { return }; draft = opened }
            pin(draft.id)
        } catch { report(error) }
    }
    func openDraftInNewWindow(_ id: UUID) {
        select(id); coordinator?.detachTab(id)
    }
    func openFileInNewWindow(_ url: URL) {
        open(url)
        if let draft = draftForFile(url) { coordinator?.detachTab(draft.id) }
    }
    @discardableResult func renameDraft(_ id: UUID, to name: String) -> Bool {
        flushEditor(); coordinator?.owner(of: id)?.flushEditor()
        guard var draft = drafts.first(where: { $0.id == id }) else { return false }
        do {
            let name = try FileService.validName(name)
            if let path = draft.sourcePath { return renameFile(URL(fileURLWithPath: path), to: name) }
            draft.displayName = name; return commit(draft)
        } catch { report(error); return false }
    }
    @discardableResult func renameFile(_ url: URL, to name: String) -> Bool {
        do { return move(url, to: try FileService.renamedDestination(for: url, name: name)) }
        catch { report(error); return false }
    }
    func deleteDraft(_ id: UUID) {
        guard let draft = drafts.first(where: { $0.id == id }) else { return }
        if let path = draft.sourcePath, FileManager.default.fileExists(atPath: path) { trashFile(URL(fileURLWithPath: path)) }
        else { trashDraft(id) }
    }
    func moveDraftPanel(_ id: UUID) {
        flushEditor(); coordinator?.owner(of: id)?.flushEditor()
        guard let draft = drafts.first(where: { $0.id == id }) else { return }
        if let path = draft.sourcePath { movePanel(URL(fileURLWithPath: path)); return }
        let panel = NSOpenPanel(); panel.canChooseFiles = false; panel.canChooseDirectories = true
        panel.canCreateDirectories = true; panel.message = "Escolha a pasta para guardar e mover este documento."
        guard panel.runModal() == .OK, let folder = panel.url else { return }
        do {
            let destination = folder.appendingPathComponent(try FileService.suggestedFilename(for: draft))
            try validateSaveDestination(draft, destination: destination)
            let saved = try FileService.save(draft, to: destination, assetsRoot: try requireStore().assetsRoot)
            if commit(saved) { if let owner = coordinator?.owner(of: id) { owner.followActiveFolder(); owner.persistSession() }; refreshExpandedFolders() }
        } catch { report(error) }
    }
    func trashDraft(_ id: UUID) {
        flushEditor()
        guard var draft = drafts.first(where: { $0.id == id }) else { return }
        draft.deletedAt = Date()
        if commit(draft, snapshot: Revision(draftID: id, content: draft.content, reason: "Antes de enviar à lixeira")) {
            (coordinator?.owner(of: id) ?? self).closeTab(id)
        }
    }
    func restoreTrash(_ id: UUID) {
        guard var draft = drafts.first(where: { $0.id == id }) else { return }
        let recoveredName = draft.title
        if draft.sourcePath != nil {
            // Restore a separate draft, never reattach an unrelated replacement
            // or recreate an original that is still in the Mac Trash.
            draft.referenceBasePath = draft.referencePath
            draft.sourcePath = nil; draft.baselineHash = nil; draft.bookmark = nil
            draft.displayName = recoveredName
        }
        draft.deletedAt = nil; draft.modifiedAt = Date()
        draft.hasWrittenContent = true // An explicit recovery also preserves a blank.
        if commit(draft) { select(id) }
    }
    func pinFolderPanel() {
        let panel = NSOpenPanel(); panel.canChooseFiles = false; panel.canChooseDirectories = true; panel.allowsMultipleSelection = true
        panel.message = "Fixar uma pasta mantém seus arquivos no lugar."
        if panel.runModal() == .OK {
            for url in panel.urls where !session.folders.contains(where: { $0.path == url.path }) {
                var folder = PinnedFolder(path: url.path)
                folder.bookmark = try? url.bookmarkData(options: [], includingResourceValuesForKeys: nil, relativeTo: nil)
                session.folders.append(folder); session.expandedPaths.append(url.path); loadChildren(url.path)
            }
            persistSession()
        }
    }
    func unpinFolder(_ id: UUID) { session.folders.removeAll { $0.id == id }; persistSession() }
    func toggleFolder(_ path: String) {
        if session.expandedPaths.contains(path) { session.expandedPaths.removeAll { $0 == path } }
        else { session.expandedPaths.append(path); loadChildren(path) }
        persistSession()
    }
    func loadChildren(_ path: String) {
        do {
            let urls = try FileManager.default.contentsOfDirectory(at: URL(fileURLWithPath: path),
                includingPropertiesForKeys: [.isDirectoryKey, .isSymbolicLinkKey], options: [])
            children[path] = urls.compactMap { url in
                let values = try? url.resourceValues(forKeys: [.isDirectoryKey, .isSymbolicLinkKey])
                let directory = values?.isDirectory == true
                return FileNode(url: url, isDirectory: directory)
            }.sorted { a, b in
                if a.isDirectory != b.isDirectory { return a.isDirectory }
                return a.name.localizedStandardCompare(b.name) == .orderedAscending
            }
            folderErrors.removeValue(forKey: path)
        } catch { folderErrors[path] = "Pasta indisponível: " + error.localizedDescription; children[path] = [] }
    }
    private func refreshExpandedFolders() {
        for path in Set(session.folders.map(\.path) + [session.currentFolder].compactMap { $0 }) { loadChildren(path) }
    }
    func createFile(in folder: URL) {
        flushEditor()
        newDraft()
        guard let draft = active else { return }
        let panel = NSSavePanel(); panel.directoryURL = folder
        panel.allowedContentTypes = [.init(filenameExtension: "md") ?? .plainText]
        panel.nameFieldStringValue = "Novo documento.md"; panel.canCreateDirectories = true
        if panel.runModal() == .OK, let url = panel.url { write(draft, destination: url, overwriteCopy: true) }
    }
    func rename(_ url: URL) {
        let alert = NSAlert(); alert.messageText = "Renomear arquivo"
        alert.informativeText = "O arquivo real será renomeado. Seu rascunho acompanhará a mudança."
        let field = NSTextField(string: url.lastPathComponent); field.frame = NSRect(x: 0, y: 0, width: 330, height: 26)
        alert.accessoryView = field; alert.addButton(withTitle: "Renomear"); alert.addButton(withTitle: "Cancelar")
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        let name = field.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty, !name.contains("/"), name != ".", name != ".." else { errorMessage = "Escolha um nome de arquivo válido."; return }
        move(url, to: url.deletingLastPathComponent().appendingPathComponent(name))
    }
    func movePanel(_ url: URL) {
        let panel = NSOpenPanel(); panel.canChooseFiles = false; panel.canChooseDirectories = true
        panel.canCreateDirectories = true; panel.message = "Escolha a pasta para mover este arquivo."
        if panel.runModal() == .OK, let folder = panel.url { move(url, to: folder.appendingPathComponent(url.lastPathComponent)) }
    }
    @discardableResult private func move(_ source: URL, to target: URL) -> Bool {
        flushEditor()
        do {
            guard !hasUnsavedMemory else { throw TextMDError.storage("Guarde as edições pendentes antes de mover ou renomear.") }
            let store = try requireStore()
            guard (try source.resourceValues(forKeys: [.isSymbolicLinkKey])).isSymbolicLink != true else {
                throw TextMDError.invalidDestination("Este item é um atalho simbólico. Mova o arquivo real ou use o Finder para preservar o destino do atalho.")
            }
            let isDirectory = (try source.resourceValues(forKeys: [.isDirectoryKey])).isDirectory == true
            let key = FileService.documentKey(source)
            let affected = drafts.filter { draft in
                guard draft.deletedAt == nil else { return false }
                guard let path = draft.sourcePath else { return false }
                let candidate = FileService.documentKey(URL(fileURLWithPath: path))
                return candidate == key || (isDirectory && candidate.hasPrefix(key + "/"))
            }
            let affectedIDs = Set(affected.map(\.id)), destinationKey = FileService.documentKey(target)
            if drafts.contains(where: { draft in
                guard draft.deletedAt == nil, !affectedIDs.contains(draft.id), let path = draft.sourcePath else { return false }
                let candidate = FileService.documentKey(URL(fileURLWithPath: path))
                return candidate == destinationKey || (isDirectory && candidate.hasPrefix(destinationKey + "/"))
            }) {
                throw TextMDError.invalidDestination("O destino já pertence a outro rascunho guardado. Escolha outro nome para preservar os dois textos.")
            }
            for draft in affected { try store.save(draft) }
            let relativePaths = Dictionary(uniqueKeysWithValues: affected.map { draft in
                (draft.id, isDirectory ? String(FileService.documentKey(URL(fileURLWithPath: draft.sourcePath!)).dropFirst(key.count)) : "")
            })
            guard source != target else { return true }
            // Existing destinations are never overwritten by a rename or move.
            if !isDirectory && FileService.isEditable(source) {
                try FileService.moveDocument(from: source, to: target, draftContents: affected.map(\.content))
            } else { try FileManager.default.moveItem(at: source, to: target) }
            for var draft in affected {
                let relative = relativePaths[draft.id] ?? ""
                let updated = URL(fileURLWithPath: target.path + relative)
                draft.sourcePath = updated.path
                draft.bookmark = try? updated.bookmarkData(options: [], includingResourceValuesForKeys: nil, relativeTo: nil)
                _ = commit(draft)
            }
            if isDirectory {
                for index in session.folders.indices {
                    let path = session.folders[index].path
                    if path == source.path || path.hasPrefix(source.path + "/") {
                        session.folders[index].path = target.path + String(path.dropFirst(source.path.count))
                        session.folders[index].bookmark = try? URL(fileURLWithPath: session.folders[index].path).bookmarkData(options: [], includingResourceValuesForKeys: nil, relativeTo: nil)
                    }
                }
                for model in coordinator?.models ?? [self] {
                    if let path = model.currentFolder, path == source.path || path.hasPrefix(source.path + "/") {
                        model.session.currentFolder = target.path + String(path.dropFirst(source.path.count))
                    }
                }
                persistSession()
            }
            for model in coordinator?.models ?? [self] { model.refreshExpandedFolders() }
            return true
        } catch { report(error); return false }
    }
    func trashFile(_ url: URL) {
        flushEditor()
        let alert = NSAlert(); alert.messageText = "Enviar \(url.lastPathComponent) à Lixeira do Mac?"
        let isDirectory = (try? url.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true
        alert.informativeText = isDirectory ? "A pasta e seus arquivos vão para a Lixeira do Mac. Os rascunhos saem da lateral e ficam na lixeira interna por 90 dias." : "O arquivo real vai para a Lixeira do Mac. Seu rascunho sai da lateral e fica recuperável na lixeira do text-md durante 90 dias."
        alert.addButton(withTitle: "Enviar à Lixeira"); alert.addButton(withTitle: "Cancelar")
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        do {
            let key = FileService.documentKey(url)
            let affected = drafts.filter { draft in
                guard draft.deletedAt == nil, let path = draft.sourcePath else { return false }
                let candidate = FileService.documentKey(URL(fileURLWithPath: path))
                return candidate == key || (isDirectory && candidate.hasPrefix(key + "/"))
            }.map(\.id)
            try FileManager.default.trashItem(at: url, resultingItemURL: nil)
            for id in affected { trashDraft(id) }
            if isDirectory {
                session.folders.removeAll { $0.path == url.path || $0.path.hasPrefix(url.path + "/") }
                for model in coordinator?.models ?? [self] {
                    if let path = model.currentFolder, path == url.path || path.hasPrefix(url.path + "/") { model.session.currentFolder = url.deletingLastPathComponent().path }
                }
                persistSession()
            }
            for model in coordinator?.models ?? [self] { model.refreshExpandedFolders(); model.checkDisk() }
        }
        catch { report(error) }
    }
    func insertImagePanel() {
        guard hasDocumentContext else { return }
        let panel = NSOpenPanel(); panel.allowedContentTypes = [.image]; panel.allowsMultipleSelection = true
        if panel.runModal() == .OK { for url in panel.urls { editor?.insertImage(url) } }
    }
    func searchFiles() {
        searchTask?.cancel(); fileResults = []; searchWarning = nil
        let query = search.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { searchRunning = false; return }
        let paths = session.folders.map(\.path)
        searchRunning = true
        searchTask = Task { [weak self] in
            do { try await Task.sleep(for: .milliseconds(250)) } catch { return }
            let worker = Task.detached(priority: .userInitiated) { try FileSearch.scan(query: query, paths: paths) }
            let output: (results: [FileSearchResult], errors: [String])
            do {
                output = try await withTaskCancellationHandler(operation: { try await worker.value }, onCancel: { worker.cancel() })
            } catch is CancellationError { return }
            catch { if !Task.isCancelled { self?.searchRunning = false; self?.report(error) }; return }
            guard !Task.isCancelled else { return }
            self?.fileResults = output.results; self?.searchRunning = false
            if !output.errors.isEmpty { self?.searchWarning = "Alguns arquivos não puderam ser lidos: " + output.errors.prefix(3).joined(separator: "; ") }
        }
    }
    func format(_ action: MarkdownAction) {
        guard hasDocumentContext, editor?.draftID == active?.id else { return }
        editor?.format(action)
    }
    func retryGuarding() {
        flushEditor()
        for draft in drafts where uncommitted.contains(draft.id) { _ = commit(draft) }
        persistSession()
    }
    func undoManager(for id: UUID) -> UndoManager {
        if let manager = undoManagers[id] { return manager }
        let manager = UndoManager(); undoManagers[id] = manager; return manager
    }
}
