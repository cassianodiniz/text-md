import Testing
import Foundation
@testable import TextMDCore

@Suite(.serialized)
struct AuditRegressionTests {
    private func folder() throws -> URL {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("textmd-regression-" + UUID().uuidString)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }
    @Test func markdownContextsReferencesAndUTF16Ranges() {
        let text = "# Olá 👨‍👩‍👧‍👦\r\n\r\n[guia][ref] e [outro](outro.md \"Título\")\r\n\r\n[ref]: docs/ação.md\r\n\r\n`**literal** [não](falso.md)` e \\*literal\\*\r\n"
        let parsed = MarkdownAnalysis(text)
        #expect(parsed.links.map(\.destination) == ["docs/ação.md", "outro.md"])
        #expect(parsed.links.map { (text as NSString).substring(with: $0.label) } == ["guia", "outro"])
        #expect(!parsed.inlines.contains { $0.kind == "strong" || $0.kind == "emphasis" })
        #expect(parsed.readableText.contains("**literal** [não](falso.md)"))
        #expect(parsed.readableText.contains("*literal*"))
        let nested = MarkdownAnalysis("````\n```\n# código\n````\n")
        #expect(nested.codeBlocks.count == 1)
        #expect(nested.headings.isEmpty)
        #expect(nested.readableText.contains("# código"))
        #expect(MarkdownAnalysis("Título\n======\n").headings.first?.level == 1)
        #expect(MarkdownAnalysis("| texto sem tabela\n").tables.isEmpty)
        #expect(MarkdownAnalysis("| A | B |\n| - | - |\n| 1 | 2 |\n").tableRows.count == 2)
    }
    @Test func unicodeLineSeparatorsDoNotShiftMarkdownOffsets() {
        let source = "😀\u{2028}![a](x.png)\n**c**"
        let parsed = MarkdownAnalysis(source), ns = source as NSString
        #expect(parsed.images.count == 1)
        #expect(ns.substring(with: parsed.images[0].range) == "![a](x.png)")
        #expect(parsed.inlines.filter { $0.kind == "strong" }.map { ns.substring(with: $0.range) } == ["**c**"])
    }
    @Test func unusualReferencesAndFilenamesExportWithoutRewritingLookalikes() throws {
        let root = try folder(); defer { try? FileManager.default.removeItem(at: root) }
        let assets = root.appendingPathComponent("assets")
        let ref = try FileService.storeImage(Data([7,8,9]), extension: "png", assetsRoot: assets)
        for (index, input) in [
            "![x][a]\n\n[a]: not-an-image invalid title\n\n[a]: \(ref)",
            "<!--\n[a]: fake.png\n-->\n\n![x][a]\n\n[a]: \(ref)",
            "![x][a]\n\n[a]:\n \(ref) \"T\"",
            "> ![x][a]\n>\n> [a]: \(ref)",
            "![x][SS]\n\n[ß]: \(ref)"
        ].enumerated() {
            let name = index.isMultiple(of: 2) ? "post(\(index).md" : "post)\(index).md"
            let saved = try FileService.save(Draft(content: input), to: root.appendingPathComponent(name), assetsRoot: assets)
            let parsed = MarkdownAnalysis(saved.content)
            #expect(parsed.images.count == 1)
            #expect(parsed.images.first?.destination.hasPrefix("textmd-asset:") == false)
            if index == 0 { #expect(saved.content.contains("[a]: not-an-image invalid title")) }
            if index == 1 { #expect(saved.content.contains("[a]: fake.png")) }
            let destination = try #require(parsed.images.first?.destination)
            let path = try #require(URLComponents(string: destination)?.path)
            #expect(FileManager.default.fileExists(atPath: root.appendingPathComponent(path).path))
        }
    }
    @Test func saveExportsOnlyRealImageDestinationsIncludingReferenceImages() throws {
        let root = try folder(); defer { try? FileManager.default.removeItem(at: root) }
        let assets = root.appendingPathComponent("assets")
        let reference = try FileService.storeImage(Data([1,2,3]), extension: "png", assetsRoot: assets)
        let input = "Exemplo literal: `\(reference)`\n\n```\n![exemplo](textmd-asset:ausente.png)\n```\n\n![Foto](\(reference) \"Minha foto\")\n\n![Outra][img]\n\n[img]: \(reference)\n"
        let saved = try FileService.save(Draft(content: input), to: root.appendingPathComponent("post.md"), assetsRoot: assets)
        #expect(saved.content.contains("`\(reference)`"))
        #expect(saved.content.contains("![exemplo](textmd-asset:ausente.png)"))
        let images = MarkdownAnalysis(saved.content).images
        #expect(images.count == 2)
        #expect(images.allSatisfy { $0.destination.hasPrefix("post.images/") })
        #expect(saved.content.contains("\"Minha foto\""))
        let literal = "Apenas `textmd-asset:ausente.png` sem imagem."
        let note = try FileService.save(Draft(content: literal), to: root.appendingPathComponent("literal.md"), assetsRoot: assets)
        #expect(note.content == literal)
    }
    @Test func movePreservesAllParsedLocalImagesAndRejectsSymlink() throws {
        let root = try folder(); defer { try? FileManager.default.removeItem(at: root) }
        for (index, content) in ["![Foto](foto.bmp)", "![Foto](foto.bmp \"Título\")", "![Foto][ref]\n\n[ref]: foto.bmp", "![Foto](foto.bmp#fragmento)"].enumerated() {
            let before = root.appendingPathComponent("before\(index)"), after = root.appendingPathComponent("after\(index)")
            try FileManager.default.createDirectory(at: before, withIntermediateDirectories: true)
            try FileManager.default.createDirectory(at: after, withIntermediateDirectories: true)
            let source = before.appendingPathComponent("nota.md"), target = after.appendingPathComponent("nota.md")
            try content.write(to: source, atomically: true, encoding: .utf8)
            try Data([4,5,6]).write(to: before.appendingPathComponent("foto.bmp"))
            try FileService.moveDocument(from: source, to: target)
            #expect(try String(contentsOf: target) == content)
            #expect(try Data(contentsOf: after.appendingPathComponent("foto.bmp")) == Data([4,5,6]))
        }
        let real = root.appendingPathComponent("real.md"), alias = root.appendingPathComponent("alias.md")
        try "Texto original".write(to: real, atomically: true, encoding: .utf8)
        try FileManager.default.createSymbolicLink(atPath: alias.path, withDestinationPath: "real.md")
        #expect(throws: TextMDError.self) { try FileService.moveDocument(from: alias, to: root.appendingPathComponent("after0/alias.md")) }
        #expect(try String(contentsOf: real) == "Texto original")
        #expect(try String(contentsOf: alias) == "Texto original")
    }
    @Test func closingPersistenceIsAtomicOnReadOnlyFailure() throws {
        let root = try folder(); defer { try? FileManager.default.removeItem(at: root) }
        let store = try DraftStore(root: root)
        var draft = Draft(content: "Texto guardado"); draft.hiddenFromSidebar = false
        var session = SessionState(); session.tabs = [draft.id]; session.selectedID = draft.id
        try store.save(draft); try store.saveSession(session)
        let readOnly = try DraftStore(root: root, readOnly: true)
        var hidden = draft; hidden.hiddenFromSidebar = true
        #expect(throws: (any Error).self) { try readOnly.saveClosing([hidden], session: SessionState()) }
        #expect(try store.allDrafts().first?.hiddenFromSidebar == false)
        #expect(try store.session() == session)
    }
    @Test func mediaCleanupRetainsActiveClosedHistoryAndFreshAssets() throws {
        let root = try folder(); defer { try? FileManager.default.removeItem(at: root) }
        let store = try DraftStore(root: root), now = Date(), old = now.addingTimeInterval(-91 * 86400)
        func image(_ byte: UInt8, date: Date) throws -> String {
            let ref = try FileService.storeImage(Data([byte]), extension: "png", assetsRoot: store.assetsRoot)
            try FileManager.default.setAttributes([.modificationDate: date], ofItemAtPath: try FileService.assetURL(ref, assetsRoot: store.assetsRoot).path)
            return ref
        }
        let expired = try image(1, date: old), active = try image(2, date: old), closed = try image(3, date: old), history = try image(4, date: old), fresh = try image(5, date: now), encoded = try image(6, date: old)
        var deleted = Draft(content: "![expirada](\(expired))", now: old); deleted.deletedAt = old
        try store.save(deleted)
        let kept = Draft(content: "![ativa](\(active))")
        try store.save(kept, snapshot: Revision(draftID: kept.id, content: "![histórico](\(history))", reason: "Teste", now: now))
        try store.save(Draft(content: "![codificada](" + encoded.replacingOccurrences(of: "textmd-asset:", with: "textmd&#45;asset:").replacingOccurrences(of: ".png", with: "&#46;png") + ")"))
        var hidden = Draft(content: "![fechada](\(closed))"); hidden.hiddenFromSidebar = true; try store.save(hidden)
        _ = try store.cleanup(now: now)
        #expect(!FileManager.default.fileExists(atPath: try FileService.assetURL(expired, assetsRoot: store.assetsRoot).path))
        for ref in [active, closed, history, fresh, encoded] { #expect(FileManager.default.fileExists(atPath: try FileService.assetURL(ref, assetsRoot: store.assetsRoot).path)) }
    }
    @Test func searchKeepsContextAndChecksCancellationInsideTraversal() throws {
        let root = try folder(); defer { try? FileManager.default.removeItem(at: root) }
        for name in ["a", "b"] {
            let dir = root.appendingPathComponent(name); try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            try "Texto procurado \(name)".write(to: dir.appendingPathComponent("SKILL.md"), atomically: true, encoding: .utf8)
        }
        let output = try FileSearch.scan(query: "procurado", paths: [root.path])
        #expect(output.results.count == 2)
        #expect(output.results[0].path != output.results[1].path)
        #expect(output.results.allSatisfy { $0.excerpt.contains("procurado") })
        var checks = 0
        #expect(throws: CancellationError.self) {
            _ = try FileSearch.scan(query: "procurado", paths: [root.path], checkCancellation: { checks += 1; if checks > 2 { throw CancellationError() } })
        }
        #expect(checks == 3)
    }
}
