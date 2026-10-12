import Testing
import Foundation
@testable import TextMDCore

struct WindowLayoutTests {
    @Test func transferPreservesIdentityAndOtherWindowsSelection() {
        let a = UUID(), b = UUID(), c = UUID()
        var first = WindowState(); first.tabs = [a, b]; first.selectedID = a
        var second = WindowState(); second.tabs = [c]; second.selectedID = c
        var layout = WindowLayout(windows: [first, second])
        let moved = layout.move(a, to: second.id, before: c)
        #expect(moved)
        #expect(layout.windows[0].tabs == [b]); #expect(layout.windows[0].selectedID == b)
        #expect(layout.windows[1].tabs == [a, c]); #expect(layout.windows[1].selectedID == a)
        #expect(layout.owner(of: a) == second.id)
        let missingDocument = layout.move(UUID(), to: second.id)
        #expect(!missingDocument)
        let missingWindow = layout.move(b, to: UUID())
        #expect(!missingWindow)
    }
    @Test func sameTabDropAndLegacyFileDuplicatesDoNotCreateCopies() {
        let a = UUID(), b = UUID(), sameFile = UUID()
        var window = WindowState(); window.tabs = [a, b]; window.selectedID = a
        var layout = WindowLayout(windows: [window])
        let unchanged = layout.move(a, to: window.id, before: a)
        #expect(unchanged); #expect(layout.windows[0].tabs == [a, b])
        var other = WindowState(); other.tabs = [sameFile]
        let restored = WindowLayout(windows: [window, other], identities: [a: "/Skills/SKILL.md", sameFile: "/Skills/SKILL.md"])
        #expect(restored.windows[1].tabs.isEmpty)
    }
    @Test func restorationRejectsDuplicateAndMissingDocuments() {
        let a = UUID(), b = UUID(), missing = UUID()
        var first = WindowState(); first.tabs = [a, a, missing]; first.selectedID = missing
        var second = WindowState(); second.tabs = [a, b]; second.selectedID = a
        let layout = WindowLayout(windows: [first, second], validIDs: [a, b])
        #expect(layout.windows[0].tabs == [a]); #expect(layout.windows[0].selectedID == a)
        #expect(layout.windows[1].tabs == [b]); #expect(layout.windows[1].selectedID == b)
    }
    @Test func multipleWindowSessionSurvivesStorageWithoutPersistingUntouchedDraft() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("textmd-windows-" + UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: root) }
        let saved = Draft(content: "# Minha skill\n"), untouched = Draft()
        var session = SessionState(); session.zoom = 1.4; session.appearance = .light
        session.currentFolder = "/tmp/Skills"; session.sidebarMode = .folders; session.collapsedSections = ["pinned"]
        var first = WindowState(session: session); first.tabs = [saved.id]; first.selectedID = saved.id
        var second = WindowState(); second.tabs = [untouched.id]; second.selectedID = untouched.id
        session.windows = WindowLayout(windows: [first, second], validIDs: [saved.id]).windows
        do {
            let store = try DraftStore(root: root); try store.save(saved); try store.saveSession(session)
        }
        let reopened = try DraftStore(root: root)
        #expect(try reopened.allDrafts().map(\.id) == [saved.id])
        let restored = try reopened.session()
        #expect(restored == session)
        #expect(restored.windows?[1].tabs.isEmpty == true)
        #expect(restored.windows?[0].currentFolder == "/tmp/Skills")
        #expect(restored.windows?[0].collapsedSections == ["pinned"])
        #expect(restored.windows?[0].applying(to: restored).zoom == 1.4)
    }
    @Test func markdownReferencesResolveRelativeAndEncodedPathsButNotRemoteURLs() {
        #expect(FileService.linkedMarkdown("references/a%20b.md#titulo", from: "/tmp/Skill/SKILL.md")?.path == "/tmp/Skill/references/a b.md")
        #expect(FileService.linkedMarkdown("../Outra/SKILL.md", from: "/tmp/Skill/SKILL.md")?.path == "/tmp/Outra/SKILL.md")
        #expect(FileService.linkedMarkdown("file:///tmp/a.md", from: nil)?.path == "/tmp/a.md")
        #expect(FileService.linkedMarkdown("https://example.com/a.md", from: "/tmp/a.md") == nil)
        #expect(FileService.linkedMarkdown("file://other-computer/tmp/a.md", from: nil) == nil)
        #expect(FileService.linkedMarkdown("#titulo", from: "/tmp/a.md") == nil)
        #expect(FileService.linkedMarkdown("a.pdf", from: "/tmp/a.md") == nil)
        #expect(FileService.isEditable(URL(fileURLWithPath: "/tmp/SKILL.md")))
        #expect(FileService.isEditable(URL(fileURLWithPath: "/tmp/LICENSE")))
        #expect(!FileService.isEditable(URL(fileURLWithPath: "/tmp/imagem.png")))
    }
}
