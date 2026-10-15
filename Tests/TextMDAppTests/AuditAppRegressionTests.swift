import Testing
import Foundation
import AppKit
import CSQLite
import TextMDCore
@testable import TextMD

@Suite(.serialized) @MainActor
struct AuditAppRegressionTests {
    private func fixture() throws -> (URL, AppLibrary, WindowCoordinator, AppModel) {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("textmd-app-regression-" + UUID().uuidString)
        let library = AppLibrary(root: root)
        let coordinator = WindowCoordinator(library: library), model = coordinator.claimWindow()
        return (root, library, coordinator, model)
    }
    @Test func trashCannotFormatAnInvisibleDocumentAndCanReturn() throws {
        let (root, library, _, model) = try fixture(); defer { model.stopObserving(); try? FileManager.default.removeItem(at: root) }
        model.newDraft(content: "Meu texto")
        let editor = MarkdownTextView(frame: .zero)
        editor.model = model; editor.delegate = editor; editor.string = "Meu texto"; editor.draftID = model.active?.id
        editor.setSelectedRange(NSRange(location: 0, length: 9)); model.editor = editor
        model.enterTrash()
        #expect(!model.hasDocumentContext)
        model.format(.bold)
        #expect(editor.string == "Meu texto")
        #expect(library.drafts.first { $0.id == model.active?.id }?.content == "Meu texto")
        model.leaveTrash()
        #expect(model.hasDocumentContext)
        #expect(model.active?.content == "Meu texto")
    }
    @Test func widthAndSearchIntentPersistWithoutAViewOrWindow() throws {
        let (root, library, coordinator, model) = try fixture(); defer { model.stopObserving(); try? FileManager.default.removeItem(at: root) }
        model.setSidebarWidth(301); coordinator.persist(from: model)
        #expect(try library.store?.session().windows?.first?.sidebarWidth == 301)
        model.sidebarVisible = false; let request = model.searchFocusRequest; model.focusSearch()
        #expect(model.sidebarVisible && model.searchVisible)
        #expect(model.searchFocusRequest == request + 1)
        let state = WindowState(id: model.windowID, session: model.session)
        #expect(state.applying(to: SessionState()).sidebarWidth == 301)
    }
    @Test func refusedWindowCloseDoesNotHideAnyDocuments() throws {
        let (root, library, coordinator, model) = try fixture(); defer { model.stopObserving(); try? FileManager.default.removeItem(at: root) }
        model.newDraft(content: "Preservar e manter visível")
        let id = try #require(model.active?.id)
        library.uncommitted.insert(UUID())
        #expect(!coordinator.prepareToClose(model))
        #expect(library.drafts.first { $0.id == id }?.hiddenFromSidebar == false)
        #expect(model.session.tabs.contains(id))
        library.uncommitted.removeAll()
        library.store = try DraftStore(root: root, readOnly: true)
        #expect(!coordinator.prepareToClose(model))
        #expect(library.drafts.first { $0.id == id }?.hiddenFromSidebar == false)
    }
    @Test func sessionFailureDuringLastTabCloseRollsBackVisibilityAndPlacement() throws {
        let (root, library, coordinator, model) = try fixture(); defer { model.stopObserving(); try? FileManager.default.removeItem(at: root) }
        model.newDraft(content: "Nunca ocultar se falhar")
        let id = try #require(model.active?.id), oldTabs = model.session.tabs
        #expect(oldTabs.count == 1)
        var db: OpaquePointer?
        #expect(sqlite3_open(root.appendingPathComponent("library.sqlite").path, &db) == SQLITE_OK)
        defer { sqlite3_close(db) }
        #expect(sqlite3_exec(db, "CREATE TRIGGER fail_session BEFORE INSERT ON settings BEGIN SELECT RAISE(FAIL, 'session test failure'); END;", nil, nil, nil) == SQLITE_OK)
        #expect(!coordinator.closeTab(id, in: model))
        #expect(model.session.tabs == oldTabs)
        #expect(library.drafts.first { $0.id == id }?.hiddenFromSidebar == false)
        #expect(try library.store?.allDrafts().first { $0.id == id }?.hiddenFromSidebar == false)
    }
    @Test func recoveringNeverAdoptsAReplacementOriginal() throws {
        let (root, library, coordinator, model) = try fixture(); defer { model.stopObserving(); try? FileManager.default.removeItem(at: root) }
        let file = root.appendingPathComponent("original.md")
        try "Texto antigo".write(to: file, atomically: true, encoding: .utf8)
        var deleted = try FileService.open(file); deleted.deletedAt = Date()
        library.drafts = [deleted]; try library.store?.save(deleted)
        try "Arquivo substituto".write(to: file, atomically: true, encoding: .utf8)
        model.restoreTrash(deleted.id)
        #expect(model.active?.content == "Texto antigo")
        #expect(model.active?.sourcePath == nil)
        #expect(model.active?.title == "original.md")
        #expect(try String(contentsOf: file) == "Arquivo substituto")
        _ = coordinator
    }
    @Test func cachedTextKitInstanceKeepsUndoAndRedoAcrossTrash() throws {
        let (root, _, coordinator, model) = try fixture(); defer { model.stopObserving(); try? FileManager.default.removeItem(at: root) }
        model.newDraft(content: "Antes")
        let draft = try #require(model.active)
        let first = MarkdownEditor.scrollView(for: draft, model: model)
        let editor = try #require(first.documentView as? MarkdownTextView)
        let manager = try #require(editor.undoManager)
        manager.beginUndoGrouping()
        editor.insertText(" mais", replacementRange: NSRange(location: 5, length: 0))
        manager.endUndoGrouping()
        model.enterTrash()
        MarkdownEditor.dismantleNSView(first, coordinator: MarkdownEditor.Coordinator(model))
        model.leaveTrash()
        let reopened = MarkdownEditor.scrollView(for: try #require(model.active), model: model)
        #expect(first === reopened)
        #expect(manager.canUndo)
        manager.undo(); editor.flush()
        #expect(editor.string == "Antes")
        #expect(model.active?.content == "Antes")
        #expect(manager.canRedo)
        manager.redo(); editor.flush()
        #expect(model.active?.content == "Antes mais")
        _ = coordinator
    }
    @Test func savingOntoAnotherDraftIsRejectedBeforeWritingEitherOriginal() throws {
        let (root, library, _, model) = try fixture(); defer { model.stopObserving(); try? FileManager.default.removeItem(at: root) }
        let a = root.appendingPathComponent("A.md"), b = root.appendingPathComponent("B.md")
        try "Original A".write(to: a, atomically: true, encoding: .utf8)
        try "Original B".write(to: b, atomically: true, encoding: .utf8)
        var draftA = try FileService.open(a); draftA.content = "Edição A"
        var draftB = try FileService.open(b); draftB.content = "Edição B"
        library.drafts = [draftA, draftB]
        try library.store?.save(draftA); try library.store?.save(draftB)
        #expect(!model.write(draftA, destination: b, overwriteCopy: true))
        #expect(try String(contentsOf: a) == "Original A")
        #expect(try String(contentsOf: b) == "Original B")
        #expect(library.drafts.map(\.content) == ["Edição A", "Edição B"])
    }
    @Test func styledCodeAndFalseTableKeepLiteralContentVisible() throws {
        let source = "`**literal** [guia](guia.md)`\n\n\\*literal\\*\n\n| sem tabela\n"
        let storage = NSTextStorage(string: source)
        MarkdownStyler.style(storage, activeRange: NSRange(location: NSNotFound, length: 0), source: false, width: 600, documentPath: nil, assetsRoot: nil)
        var links = 0, tables = 0
        storage.enumerateAttribute(.link, in: NSRange(location: 0, length: storage.length)) { value, _, _ in if value != nil { links += 1 } }
        storage.enumerateAttribute(.tablePreview, in: NSRange(location: 0, length: storage.length)) { value, _, _ in if value != nil { tables += 1 } }
        #expect(links == 0); #expect(tables == 0); #expect(storage.string == source)
        let literal = (source as NSString).range(of: "**literal**")
        let font = try #require(storage.attribute(.font, at: literal.location, effectiveRange: nil) as? NSFont)
        #expect(font.pointSize >= 14)
    }
}
