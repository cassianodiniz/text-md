import Foundation

public struct DocumentCounts: Equatable, Sendable {
    public let lines: Int
    public let words: Int
    public let characters: Int
    public init(markdown: String, ignoreMarkdown: Bool) {
        let breaks = (try? NSRegularExpression(pattern: "\\r\\n|[\\r\\n]"))?
            .numberOfMatches(in: markdown, range: NSRange(location: 0, length: markdown.utf16.count)) ?? 0
        lines = markdown.isEmpty ? 0 : breaks + 1
        let measured = ignoreMarkdown ? ReadableMarkdown.text(markdown) : markdown
        characters = measured.count
        var count = 0
        measured.enumerateSubstrings(in: measured.startIndex..<measured.endIndex, options: .byWords) { _, _, _, _ in count += 1 }
        words = count
    }
}

/// A measurement projection, never a rewrite of the document. Code and visible
/// front matter remain readable text; only formatting and hidden destinations go.
public enum ReadableMarkdown {
    public static func text(_ markdown: String) -> String { MarkdownAnalysis(markdown).readableText }
}
