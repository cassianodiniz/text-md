import Testing
import Foundation
@testable import TextMDCore

@Suite(.serialized)
struct EditorFeaturesTests {
    @Test func tokenCountsMatchOfficialTokenizerIncludingPortugueseAndMarkdown() throws {
        struct Example: Decodable { let text: String; let counts: [String: Int] }
        let url = try #require(Bundle.module.url(forResource: "tokenizer-reference", withExtension: "json"))
        let examples = try JSONDecoder().decode([Example].self, from: Data(contentsOf: url))
        for profile in TokenProfile.allCases {
            let tokenizer = try LocalTokenizer(profile: profile)
            for example in examples {
                #expect(try tokenizer.count(example.text) == example.counts[profile.rawValue], "\(profile.rawValue): \(example.text.prefix(80))")
            }
        }
    }
    @Test func countersKeepSourceLinesAndSupportRawAndReadableText() {
        let source = "# Olá\r\n\r\nLeia **isto** e [a referência](https://example.com/a(b))."
        let raw = DocumentCounts(markdown: source, ignoreMarkdown: false)
        let readable = DocumentCounts(markdown: source, ignoreMarkdown: true)
        #expect(raw.lines == 3); #expect(readable.lines == 3)
        #expect(readable.words == 6)
        #expect(readable.characters < raw.characters)
        #expect(ReadableMarkdown.text("`**literal**` e \\*texto\\*") == "**literal** e *texto*")
        #expect(ReadableMarkdown.text("`[texto](link)` e [link](https://a(b))") == "[texto](link) e link")
        #expect(DocumentCounts(markdown: "👨‍👩‍👧‍👦", ignoreMarkdown: false).characters == 1)
        #expect(DocumentCounts(markdown: "", ignoreMarkdown: true).lines == 0)
    }
    @Test func diffTracksReplacementsAndEndDeletionsWithoutChangingCurrentText() {
        let old = "# Skill\r\nLeia o antigo.\r\nFim."
        let current = "# Skill\r\nLeia o novo.\r\n"
        let diff = InlineDiff(original: old, current: current)
        #expect(diff.deletions.map(\.text).joined() == "Leia o antigo.\r\nFim.")
        #expect(diff.deletions.first?.anchor == "# Skill\r\n".utf16.count)
        let tail = InlineDiff(original: "Primeira\r\nÚltima", current: "Primeira\r\n")
        #expect(tail.deletions == [DiffDeletion(anchor: "Primeira\r\n".utf16.count, text: "Última")])
        #expect(diff.additions.map { (current as NSString).substring(with: $0) }.joined() == "Leia o novo.\r\n")
        #expect(InlineDiff(original: current, current: current).additions.isEmpty)
        #expect(InlineDiff(original: current, current: current).deletions.isEmpty)
        #expect(InlineDiff(original: old, current: "").deletions == [DiffDeletion(anchor: 0, text: old)])
        #expect(InlineDiff(original: "", current: current).additions.reduce(0) { $0 + $1.length } == current.utf16.count)
    }
    @Test func oldSessionDecodesAndNewPreferencesSurviveReopening() throws {
        let old = Data(#"{"tabs":[],"folders":[],"sourceMode":false,"showLineNumbers":false,"expandedPaths":[]}"#.utf8)
        let decoded = try JSONDecoder().decode(SessionState.self, from: old)
        #expect(decoded.appearance == nil); #expect(decoded.metrics == nil)
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("text-md-settings-" + UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: root) }
        var session = decoded
        session.appearance = .light; session.metrics = DocumentMetric.allCases
        session.ignoreMarkdown = false; session.tokenProfile = .cl100k; session.diffEnabled = true
        do { let store = try DraftStore(root: root); try store.saveSession(session) }
        #expect(try DraftStore(root: root).session() == session)
    }
}
