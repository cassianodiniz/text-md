import Testing
import Foundation
import AppKit
import TextMDCore
@testable import TextMD

@Suite(.serialized) @MainActor
struct LifecycleRegressionTests {
    private func fixture() -> (URL, AppLibrary, WindowCoordinator, AppModel) {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("textmd-lifecycle-" + UUID().uuidString)
        let library = AppLibrary(root: root), coordinator = WindowCoordinator(library: library)
        return (root, library, coordinator, coordinator.claimWindow())
    }
    @Test func closeTabKeepsEveryWrittenDraftInLibrary() throws {
        let (root, library, coordinator, model) = fixture()
        defer { model.stopObserving(); try? FileManager.default.removeItem(at: root) }
        for n in 1...3 { model.newDraft(content: "Rascunho \(n)") }
        let ids = model.tabs.filter { !$0.content.isEmpty }.map(\.id)
        for id in ids.reversed() { #expect(coordinator.closeTab(id, in: model)) }
        #expect(model.visibleDrafts.count == 3)
        #expect(try library.store!.allDrafts().map(\.content).sorted() == (1...3).map { "Rascunho \($0)" })
        let reopened = AppLibrary(root: root)
        #expect(reopened.drafts.filter { $0.visibleInSidebar(openIDs: []) }.count == 3)
    }
    @Test func emptySessionDoesNotManufactureAnEditorTab() {
        let (root, _, _, model) = fixture()
        defer { model.stopObserving(); try? FileManager.default.removeItem(at: root) }
        #expect(model.session.tabs.isEmpty)
        #expect(model.active == nil)
    }
    @Test func incomingSelectionRespectsPendingWindowReservation() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("textmd-lifecycle-" + UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: root) }
        let store = try DraftStore(root: root), a = Draft(content: "A"), b = Draft(content: "B")
        try store.save(a); try store.save(b)
        var first = WindowState(); first.tabs = [a.id]; first.selectedID = a.id
        var second = WindowState(); second.tabs = [b.id]; second.selectedID = b.id
        var session = SessionState(); session.windows = [first, second]; try store.saveSession(session)
        let library = AppLibrary(root: root), coordinator = WindowCoordinator(library: library)
        let firstModel = coordinator.claimWindow(); defer { firstModel.stopObserving() }
        firstModel.select(b.id)
        let secondModel = coordinator.claimWindow(); defer { secondModel.stopObserving() }
        #expect(!firstModel.session.tabs.contains(b.id))
        #expect(secondModel.session.tabs == [b.id])
    }
    @Test func lastRealTabCloseDockAndWindowRestoreUseOneLibraryWindow() throws {
        let (root, library, coordinator, model) = fixture()
        defer { for model in coordinator.models { model.stopObserving() }; try? FileManager.default.removeItem(at: root) }
        model.newDraft(content: "Última aba escrita"); let id = model.active!.id
        #expect(model.session.tabs == [id])
        #expect(coordinator.closeTab(id, in: model)); coordinator.completeClose(model)
        #expect(coordinator.models.isEmpty)
        #expect(try library.store!.session().windows?.isEmpty == true)
        coordinator.reopenApplication(); coordinator.reopenApplication()
        #expect(coordinator.pendingWindowCount == 1)
        let browser = coordinator.claimWindow()
        #expect(browser.showingLibrary && browser.session.tabs.isEmpty)
        #expect(browser.visibleDrafts.map(\.id) == [id])
        coordinator.restoreLastWindow()
        #expect(coordinator.models.count == 1 && coordinator.pendingWindowCount == 0)
        #expect(browser.session.tabs == [id] && browser.active?.content == "Última aba escrita")
        #expect(browser.hasDocumentContext && browser.visibleDrafts.map(\.id) == [id])
    }
    @Test func wholeWindowRestoresOrderSelectionAndModesAfterProcessRestart() throws {
        let (root, library, coordinator, model) = fixture()
        defer { for model in coordinator.models { model.stopObserving() }; try? FileManager.default.removeItem(at: root) }
        model.newDraft(content: "A"); let a = model.active!.id
        model.newDraft(content: "B"); let b = model.active!.id
        model.select(a); model.session.sourceMode = true; model.setSidebarWidth(307)
        #expect(coordinator.prepareToClose(model)); coordinator.completeClose(model)
        #expect(try library.store!.allDrafts().count == 2)
        let restoredLibrary = AppLibrary(root: root), restoredCoordinator = WindowCoordinator(library: restoredLibrary)
        let browser = restoredCoordinator.claimWindow(); defer { browser.stopObserving() }
        #expect(browser.session.tabs.isEmpty && browser.visibleDrafts.count == 2)
        restoredCoordinator.restoreLastWindow()
        #expect(browser.session.tabs == [a, b] && browser.active?.id == a)
        #expect(browser.session.sourceMode && browser.sidebarWidth == 307)
        #expect(restoredCoordinator.closedItems.isEmpty)
    }
    @Test func quitSnapshotKeepsEveryOpenWindowAndDoesNotRestoreExplicitlyClosedTabs() throws {
        let (root, library, coordinator, first) = fixture()
        defer { for model in coordinator.models { model.stopObserving() }; try? FileManager.default.removeItem(at: root) }
        first.newDraft(content: "Primeiro"); let a = first.active!.id
        first.newDraft(content: "Fechado de propósito"); let closed = first.active!.id
        #expect(coordinator.closeTab(closed, in: first))
        let second = coordinator.claimWindow(); second.newDraft(content: "Segunda janela"); let b = second.active!.id
        coordinator.flushAll()
        let saved = try library.store!.session()
        #expect(saved.windows?.map(\.tabs) == [[a], [b]])
        #expect(saved.closedItems?.last?.window.tabs == [closed])
        let nextLibrary = AppLibrary(root: root), next = WindowCoordinator(library: nextLibrary)
        let nextA = next.claimWindow(), nextB = next.claimWindow()
        defer { nextA.stopObserving(); nextB.stopObserving() }
        #expect(nextA.session.tabs == [a] && nextB.session.tabs == [b])
        #expect(nextA.visibleDrafts.count == 3)
    }
    @Test func closingActiveUsesAdjacentTabAndClosingBackgroundKeepsSelection() throws {
        let (root, _, coordinator, model) = fixture()
        defer { model.stopObserving(); try? FileManager.default.removeItem(at: root) }
        for text in ["A", "B", "C"] { model.newDraft(content: text) }
        let ids = model.session.tabs; model.select(ids[0])
        #expect(coordinator.closeTab(ids[0], in: model))
        #expect(model.session.selectedID == ids[1])
        #expect(coordinator.closeTab(ids[2], in: model))
        #expect(model.session.selectedID == ids[1] && model.active?.content == "B")
    }
    @Test func recentlyClosedOrderSurvivesQuitWithNoWindows() throws {
        let (root, _, coordinator, model) = fixture()
        defer { try? FileManager.default.removeItem(at: root) }
        model.newDraft(content: "Editado primeiro, fechado por último"); let first = model.active!.id
        model.newDraft(content: "Editado por último, fechado primeiro"); let second = model.active!.id
        #expect(coordinator.closeTab(second, in: model)); #expect(coordinator.closeTab(first, in: model))
        coordinator.completeClose(model); coordinator.flushAll()
        let library = AppLibrary(root: root), next = WindowCoordinator(library: library), browser = next.claimWindow()
        defer { browser.stopObserving() }
        next.reopenClosed()
        #expect(browser.active?.id == first)
        #expect(next.closedItems.last?.window.tabs == [second])
    }
    @Test func detachingLastTabReservesItAndDoesNotRecordAUserClosedWindow() throws {
        let (root, _, coordinator, source) = fixture()
        defer { for model in coordinator.models { model.stopObserving() }; try? FileManager.default.removeItem(at: root) }
        source.newDraft(content: "Movido, não fechado"); let id = source.active!.id
        coordinator.detachTab(id); coordinator.completeClose(source)
        #expect(coordinator.closedItems.isEmpty && coordinator.pendingWindowCount == 1)
        let libraryWindow = coordinator.claimWindow()
        #expect(libraryWindow.session.tabs == [id] && libraryWindow.hasDocumentContext)
    }
    @Test func duplicateLegacyOriginalsBecomeIndependentRecoverableDrafts() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("textmd-lifecycle-" + UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: root) }
        let store = try DraftStore(root: root), original = root.appendingPathComponent("SKILL.md")
        let bytes = Data("Original intacto".utf8); try bytes.write(to: original)
        let a = Draft(content: "Edição A", sourcePath: original.path), b = Draft(content: "Edição B", sourcePath: original.path)
        try store.save(a); try store.save(b)
        var window = WindowState(); window.tabs = [a.id, b.id]; window.selectedID = b.id
        var session = SessionState(); session.windows = [window]; try store.saveSession(session)
        let library = AppLibrary(root: root), coordinator = WindowCoordinator(library: library), model = coordinator.claimWindow()
        defer { model.stopObserving() }
        #expect(library.drafts.filter { $0.sourcePath != nil }.count == 1)
        #expect(Set(library.drafts.map(\.content)) == ["Edição A", "Edição B"])
        model.select(a.id); #expect(model.active?.content == "Edição A")
        model.select(b.id); #expect(model.active?.content == "Edição B")
        #expect(try Data(contentsOf: original) == bytes)
    }
    @Test func externalReloadSynchronizesCachedEditorBeforePositionFlush() throws {
        let (root, _, coordinator, model) = fixture()
        defer { for model in coordinator.models { model.stopObserving() }; try? FileManager.default.removeItem(at: root) }
        let file = root.appendingPathComponent("texto.md"); try Data("Anterior".utf8).write(to: file)
        model.open(file)
        let scroll = MarkdownEditor.scrollView(for: try #require(model.active), model: model)
        let editor = try #require(scroll.documentView as? MarkdownTextView)
        try Data("Mudança externa".utf8).write(to: file); model.checkDisk()
        editor.setSelectedRange(NSRange(location: 1, length: 0)); editor.flush()
        #expect(editor.string == "Mudança externa" && model.active?.content == "Mudança externa")
        #expect(try Data(contentsOf: file) == Data("Mudança externa".utf8))
    }
    @Test func failedVersionRestoreRetainsBackupAndCannotBeOverwrittenByStalePositionFlush() throws {
        let (root, library, coordinator, model) = fixture()
        defer { for model in coordinator.models { model.stopObserving() }; try? FileManager.default.removeItem(at: root) }
        model.newDraft(content: "A: texto recente sem outra versão"); let id = model.active!.id
        let scroll = MarkdownEditor.scrollView(for: try #require(model.active), model: model)
        let editor = try #require(scroll.documentView as? MarkdownTextView)
        library.store = try DraftStore(root: root, readOnly: true)
        model.restore(Revision(draftID: id, content: "B: versão restaurada", reason: "Fixture"))
        #expect(library.uncommitted.contains(id))
        editor.setSelectedRange(NSRange(location: 1, length: 0)); editor.flush()
        #expect(model.active?.content == "B: versão restaurada")
        library.store = try DraftStore(root: root); model.retryGuarding()
        #expect(library.uncommitted.isEmpty)
        #expect(try library.store!.allDrafts().first { $0.id == id }?.content == "B: versão restaurada")
        #expect(try library.store!.revisions(for: id).contains { $0.content == "A: texto recente sem outra versão" })
    }
    @Test func recoveredCopyRetainsReferenceBaseAndExportsImagesWithoutWritingOriginal() throws {
        let (root, library, coordinator, model) = fixture()
        defer { for model in coordinator.models { model.stopObserving() }; try? FileManager.default.removeItem(at: root) }
        let folder = root.appendingPathComponent("originais"), target = root.appendingPathComponent("copia")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: target, withIntermediateDirectories: true)
        let original = folder.appendingPathComponent("SKILL.md"), image = folder.appendingPathComponent("img.bmp")
        let bytes = Data("Original que não muda".utf8), imageBytes = Data([1, 2, 3, 4])
        try bytes.write(to: original); try imageBytes.write(to: image)
        var recovered = Draft(content: "![Imagem](img.bmp)\n[Guia](guia.md)")
        recovered.referenceBasePath = original.path
        try library.store!.save(recovered)
        let saved = try FileService.save(recovered, to: target.appendingPathComponent("Copia.md"), assetsRoot: library.store!.assetsRoot)
        #expect(saved.referenceBasePath == nil)
        #expect(try Data(contentsOf: target.appendingPathComponent("img.bmp")) == imageBytes)
        #expect(try Data(contentsOf: original) == bytes)
        #expect(FileService.linkedMarkdown("guia.md", from: recovered.referencePath)?.path == folder.appendingPathComponent("guia.md").path)
        _ = model
    }
    @Test func restoringWindowRetainsItsNavigationInsteadOfReplacingItWithActiveFileFolder() throws {
        let (root, _, coordinator, model) = fixture()
        defer { for model in coordinator.models { model.stopObserving() }; try? FileManager.default.removeItem(at: root) }
        let file = root.appendingPathComponent("Arquivo.md"); try Data("Texto".utf8).write(to: file)
        model.open(file); let other = root.appendingPathComponent("Outra pasta")
        try FileManager.default.createDirectory(at: other, withIntermediateDirectories: true)
        model.navigateFolder(other)
        #expect(coordinator.prepareToClose(model)); coordinator.completeClose(model)
        coordinator.reopenApplication(); let browser = coordinator.claimWindow()
        coordinator.restoreLastWindow()
        #expect(browser.currentFolder == other.path && browser.sidebarMode == .folders)
    }
    @Test func movingLastTabCommitsOwnershipAndDocumentViewModeBeforeClosingSource() throws {
        let (root, library, coordinator, source) = fixture()
        defer { for model in coordinator.models { model.stopObserving() }; try? FileManager.default.removeItem(at: root) }
        source.newDraft(content: "Mover com identidade estável"); let id = source.active!.id
        let target = coordinator.claimWindow()
        coordinator.moveTab(id, to: target); coordinator.completeClose(source)
        #expect(coordinator.models.count == 1 && target.session.tabs == [id])
        #expect(target.hasDocumentContext && coordinator.closedItems.isEmpty)
        let session = try library.store!.session()
        #expect(session.windows?.map(\.tabs) == [[id]])
        #expect(session.windows?.first?.libraryVisible == false)
    }
    @Test func renamingCannotTakePathReservedByAnotherDraftWhoseOriginalIsMissing() throws {
        let (root, library, coordinator, model) = fixture()
        defer { for model in coordinator.models { model.stopObserving() }; try? FileManager.default.removeItem(at: root) }
        let a = root.appendingPathComponent("A.md"), b = root.appendingPathComponent("B.md")
        try Data("Original A".utf8).write(to: a)
        model.open(a)
        let missing = Draft(content: "Rascunho B que deve continuar acessível", sourcePath: b.path)
        library.drafts.append(missing); try library.store!.save(missing)
        #expect(!model.renameFile(a, to: "B.md"))
        #expect(FileManager.default.fileExists(atPath: a.path))
        #expect(!FileManager.default.fileExists(atPath: b.path))
        #expect(library.drafts.first { $0.id == missing.id }?.sourcePath == b.path)
    }
}
