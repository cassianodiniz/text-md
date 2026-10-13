import Foundation
import Testing
@testable import TextMDCore

struct SidebarLifecycleTests {
    private func root() throws -> URL {
        let path = FileManager.default.temporaryDirectory.appendingPathComponent("textmd-sidebar-" + UUID().uuidString)
        try FileManager.default.createDirectory(at: path, withIntermediateDirectories: true)
        return path
    }
    @Test func closedDraftSurvivesWithoutReturningToSidebar() throws {
        let folder = try root(); defer { try? FileManager.default.removeItem(at: folder) }
        var draft = Draft(content: "# Meu texto\n\nAinda não salvo em arquivo.")
        draft.displayName = "Postagem"; draft.hiddenFromSidebar = true
        let store = try DraftStore(root: folder); try store.save(draft)
        let reopened = try DraftStore(root: folder).allDrafts()
        #expect(reopened.count == 1)
        #expect(reopened[0].content == draft.content)
        #expect(reopened[0].title == "Postagem")
        #expect(!reopened[0].visibleInSidebar(openIDs: []))
        var recovered = reopened[0]; recovered.hiddenFromSidebar = false
        #expect(recovered.visibleInSidebar(openIDs: []))
    }
    @Test func migrationKeepsOpenAndPinnedButHidesOldClosedDocuments() throws {
        let folder = try root(); defer { try? FileManager.default.removeItem(at: folder) }
        let source = folder.appendingPathComponent("Original.md")
        let bytes = Data("# Original\n".utf8); try bytes.write(to: source)
        let closed = try FileService.open(source)
        let open = Draft(content: "Estou escrevendo.")
        var pinned = Draft(content: "Fixado"); pinned.pinned = true
        let store = try DraftStore(root: folder.appendingPathComponent("library"))
        for draft in [closed, open, pinned] { try store.save(draft) }
        var session = SessionState(); session.tabs = [open.id]
        try store.hideLegacyClosedDrafts(session: session)
        try store.hideLegacyClosedDrafts(session: session)
        let values = try store.allDrafts()
        #expect(values.count == 3)
        #expect(values.first { $0.id == closed.id }?.hiddenFromSidebar == true)
        #expect(values.first { $0.id == open.id }?.hiddenFromSidebar == false)
        #expect(values.first { $0.id == pinned.id }?.hiddenFromSidebar == false)
        #expect(try Data(contentsOf: source) == bytes)
        var hidden = values.first { $0.id == closed.id }!; hidden.pinned = true
        try Data("# Alterado por fora\n".utf8).write(to: source)
        let updated = try FileService.reload(hidden)
        #expect(updated.hiddenFromSidebar == true)
        #expect(!updated.visibleInSidebar(openIDs: []))
        #expect(updated.id == hidden.id)
    }
    @Test func filenameRenameMovesTheFileWithoutRewritingOrOverwritingIt() throws {
        let folder = try root(); defer { try? FileManager.default.removeItem(at: folder) }
        let source = folder.appendingPathComponent("Velho.md")
        let bytes = Data([0xef, 0xbb, 0xbf]) + Data("# Conteúdo\r\n".utf8)
        try bytes.write(to: source)
        let destination = try FileService.renamedDestination(for: source, name: "Novo nome")
        #expect(destination.lastPathComponent == "Novo nome.md")
        try FileService.moveDocument(from: source, to: destination)
        #expect(!FileManager.default.fileExists(atPath: source.path))
        #expect(try Data(contentsOf: destination) == bytes)
        #expect(try FileService.open(destination).title == "Novo nome.md")
        try Data("Outro arquivo".utf8).write(to: source)
        #expect(throws: TextMDError.self) { try FileService.moveDocument(from: destination, to: source) }
        #expect(try Data(contentsOf: destination) == bytes)
        let upper = folder.appendingPathComponent("NOVO NOME.md")
        try FileService.moveDocument(from: destination, to: upper)
        #expect(try Data(contentsOf: upper) == bytes)
        #expect(throws: TextMDError.self) { try FileService.renamedDestination(for: upper, name: "../fora") }
    }
    @Test func metadataIsNotTheDraftNameAndUntouchedNotesDoNotClutterSidebar() throws {
        let draft = Draft(content: "---\nname: skill\ndescription: Exemplo\n---\n\n# Implementar\n")
        #expect(draft.title == "Implementar")
        var blank = Draft(); blank.displayName = "Ainda vazio"
        #expect(!blank.shouldPersist)
        #expect(!blank.visibleInSidebar(openIDs: [blank.id]))
        #expect(try FileService.suggestedFilename(for: draft) == "Implementar.md")
    }
}
