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
    public static func text(_ markdown: String) -> String {
        var inFence: String?
        var inFrontMatter = false
        let lines = markdown.components(separatedBy: "\n")
        return lines.enumerated().map { index, line in
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            if index == 0 && trimmed == "---" { inFrontMatter = true; return "" }
            if inFrontMatter {
                if trimmed == "---" || trimmed == "..." { inFrontMatter = false; return "" }
                return line
            }
            if let fence = inFence {
                if trimmed.hasPrefix(fence) { inFence = nil; return "" }
                return line
            }
            if trimmed.hasPrefix("```") || trimmed.hasPrefix("~~~") {
                inFence = String(trimmed.prefix(3)); return ""
            }
            if line.hasPrefix("    ") || line.hasPrefix("\t") { return line }
            if trimmed.range(of: "^([-*_]\\s*){3,}$", options: .regularExpression) != nil { return "" }
            if trimmed.range(of: "^\\|?[ :|-]+\\|[ :|-]*$", options: .regularExpression) != nil { return "" }
            if trimmed.range(of: "^\\[[^\\]]+\\]:\\s*\\S+", options: .regularExpression) != nil { return "" }
            var result = replace(line, "^\\s{0,3}#{1,6}\\s+", "")
            result = replace(result, "^\\s*(?:>\\s*)+", "")
            result = replace(result, "^\\s*(?:[-*+]|[0-9]+[.)])\\s+(?:\\[[ xX]\\]\\s+)?", "")
            guard let parsed = try? AttributedString(markdown: result,
                options: .init(interpretedSyntax: .inlineOnlyPreservingWhitespace)) else { return result }
            return parsed.runs.map { run in
                let value = String(parsed[run.range].characters)
                if run.inlinePresentationIntent?.contains(.code) == true { return value }
                return replace(value, "<[^>]+>", "")
            }.joined()
        }.joined(separator: "\n")
    }
    private static func replace(_ text: String, _ pattern: String, _ template: String) -> String {
        (try? NSRegularExpression(pattern: pattern))?.stringByReplacingMatches(in: text,
            range: NSRange(location: 0, length: text.utf16.count), withTemplate: template) ?? text
    }
}
