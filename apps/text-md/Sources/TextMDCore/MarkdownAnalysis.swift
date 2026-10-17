import Foundation
import Markdown

/// One CommonMark/GFM interpretation for display, measurement and file dependencies.
/// Source offsets always refer to the original UTF-16 string, never rewritten text.
public struct MarkdownAnalysis {
    public struct Span {
        public let range: NSRange
        public let inner: NSRange
        public let kind: String
        public let level: Int
    }
    public struct Reference {
        public let range: NSRange
        public let label: NSRange
        public let destination: String
        public let destinationRange: NSRange?
        public let title: String?
    }
    public struct TableRow {
        public let range: NSRange
        public let cells: [String]
        public let header: Bool
    }
    public private(set) var codeBlocks: [NSRange] = []
    public private(set) var codeMarkers: [NSRange] = []
    public private(set) var inlines: [Span] = []
    public private(set) var headings: [Span] = []
    public private(set) var setextMarkers: [NSRange] = []
    public private(set) var tables: [NSRange] = []
    public private(set) var tableRows: [TableRow] = []
    public private(set) var tableSeparators: [NSRange] = []
    public private(set) var links: [Reference] = []
    public private(set) var images: [Reference] = []
    public private(set) var escapes: [NSRange] = []
    public let readableText: String
    public let frontMatter: NSRange?

    public init(_ source: String) {
        let ns = source as NSString
        var parseSource = source
        var front: NSRange?
        var frontText = ""
        if source.hasPrefix("---\n") || source.hasPrefix("---\r") {
            var offset = NSMaxRange(ns.lineRange(for: NSRange(location: 0, length: 0)))
            let innerStart = offset
            while offset < ns.length {
                let line = ns.lineRange(for: NSRange(location: offset, length: 0))
                let value = ns.substring(with: line).trimmingCharacters(in: .whitespacesAndNewlines)
                if value == "---" || value == "..." {
                    front = NSRange(location: 0, length: NSMaxRange(line))
                    frontText = ns.substring(with: NSRange(location: innerStart, length: offset - innerStart))
                    break
                }
                offset = NSMaxRange(line)
            }
        }
        frontMatter = front
        if let front, let range = Range(front, in: source) {
            // Preserve UTF-8 columns/lines while preventing YAML from becoming Markdown.
            let masked = source[range].utf8.map { $0 == 10 || $0 == 13 ? $0 : UInt8(32) }
            parseSource.replaceSubrange(range, with: String(decoding: masked, as: UTF8.self))
        }
        let document = Document(parsing: parseSource)
        let mapper = SourceOffsets(source)
        func visible(_ node: Markup) -> String {
            if let value = node as? Markdown.Text { return value.string }
            if let value = node as? InlineCode { return value.code }
            if let value = node as? CodeBlock { return value.code }
            if node is SoftBreak || node is LineBreak { return "\n" }
            if node is InlineHTML || node is HTMLBlock || node is ThematicBreak { return "" }
            let parts = node.children.map { visible($0) }
            return parts.joined(separator: node is any TableCellContainer ? "\t" : node is BlockContainer || node is Markdown.Table ? "\n" : "")
        }
        readableText = frontText + visible(document)
        func childRange(_ node: Markup, fallback: NSRange) -> NSRange {
            let ranges = node.children.compactMap { mapper.range($0.range) }
            guard let first = ranges.first, let last = ranges.last else { return fallback }
            return NSRange(location: first.location, length: max(0, NSMaxRange(last) - first.location))
        }
        var htmlBlocks: [NSRange] = []
        func walk(_ node: Markup) {
            guard let range = mapper.range(node.range) else { for child in node.children { walk(child) }; return }
            if node is HTMLBlock { htmlBlocks.append(range); return }
            if node is CodeBlock {
                codeBlocks.append(range)
                let firstLine = ns.lineRange(for: NSRange(location: range.location, length: 0))
                let opening = ns.substring(with: firstLine).trimmingCharacters(in: .whitespacesAndNewlines)
                if let marker = opening.first, marker == "`" || marker == "~" {
                    let count = opening.prefix { $0 == marker }.count
                    if count >= 3 {
                        codeMarkers.append(firstLine)
                        var offset = NSMaxRange(firstLine)
                        while offset < NSMaxRange(range) {
                            let line = ns.lineRange(for: NSRange(location: offset, length: 0))
                            let raw = ns.substring(with: line).trimmingCharacters(in: .whitespacesAndNewlines)
                            if raw.count >= count, raw.allSatisfy({ $0 == marker }) { codeMarkers.append(line) }
                            offset = NSMaxRange(line)
                        }
                    }
                }
                return
            }
            if let code = node as? InlineCode {
                let raw = ns.substring(with: range)
                let ticks = raw.prefix { $0 == "`" }.utf16.count
                let inner = NSRange(location: range.location + ticks, length: max(0, range.length - 2 * ticks))
                inlines.append(Span(range: range, inner: inner, kind: "code", level: 0))
                _ = code; return
            }
            if node is Strong || node is Emphasis || node is Strikethrough {
                inlines.append(Span(range: range, inner: childRange(node, fallback: range), kind: node is Strong ? "strong" : node is Emphasis ? "emphasis" : "strike", level: 0))
            }
            if let heading = node as? Heading {
                headings.append(Span(range: range, inner: childRange(node, fallback: range), kind: "heading", level: heading.level))
                let first = ns.lineRange(for: NSRange(location: range.location, length: 0))
                let next = NSMaxRange(first)
                if next < NSMaxRange(range), next < ns.length {
                    let marker = ns.lineRange(for: NSRange(location: next, length: 0))
                    if ns.substring(with: marker).trimmingCharacters(in: .whitespacesAndNewlines).range(of: "^(=+|-+)$", options: .regularExpression) != nil { setextMarkers.append(marker) }
                }
            }
            if let table = node as? Markdown.Table {
                tables.append(range)
                if let headRange = mapper.range(table.head.range) {
                    tableRows.append(TableRow(range: headRange, cells: table.head.cells.map { $0.plainText }, header: true))
                    let first = ns.lineRange(for: NSRange(location: range.location, length: 0))
                    if NSMaxRange(first) < ns.length { tableSeparators.append(ns.lineRange(for: NSRange(location: NSMaxRange(first), length: 0))) }
                }
                for row in table.body.rows {
                    if let rowRange = mapper.range(row.range) { tableRows.append(TableRow(range: rowRange, cells: row.cells.map { $0.plainText }, header: false)) }
                }
            }
            if let link = node as? Link, let destination = link.destination {
                links.append(Reference(range: range, label: childRange(node, fallback: range), destination: destination, destinationRange: Self.inlineDestination(in: ns, range: range), title: link.title))
            }
            if let image = node as? Markdown.Image, let destination = image.source {
                images.append(Reference(range: range, label: childRange(node, fallback: range), destination: destination, destinationRange: Self.inlineDestination(in: ns, range: range), title: image.title))
            }
            if node is Markdown.Text {
                let raw = ns.substring(with: range)
                let regex = try! NSRegularExpression(pattern: ##"\\[!"#$%&'()*+,\-./:;<=>?@\[\]\\^_`{|}~]"##)
                for match in regex.matches(in: raw, range: NSRange(location: 0, length: (raw as NSString).length)) {
                    escapes.append(NSRange(location: range.location + match.range.location, length: 1))
                }
            }
            for child in node.children { walk(child) }
        }
        walk(document)
        // Reference definitions aren't emitted as nodes. Resolve only destinations of
        // actual parsed images; code examples and unused lookalikes are never exports.
        let protected = codeBlocks + htmlBlocks + inlines.filter { $0.kind == "code" }.map(\.range) + (front.map { [$0] } ?? [])
        let definition = try! NSRegularExpression(pattern: #"(?m)^ {0,3}\[([^\]\r\n]+)\]:[ \t]*(<[^>\r\n]*>|[^ \t\r\n]+)"#)
        let definitions = definition.matches(in: source, range: NSRange(location: 0, length: ns.length)).filter { match in !protected.contains { NSIntersectionRange($0, match.range).length > 0 } }
        images = images.map { image in
            guard image.destinationRange == nil else { return image }
            let raw = ns.substring(with: image.range)
            let label: String
            if let close = raw.lastIndex(of: "["), raw.hasSuffix("]"), close != raw.index(after: raw.startIndex) {
                let value = String(raw[raw.index(after: close)..<raw.index(before: raw.endIndex)])
                label = value.isEmpty ? ns.substring(with: image.label) : value
            } else { label = ns.substring(with: image.label) }
            let normalize: (String) -> String = { $0.split(whereSeparator: { $0.isWhitespace }).joined(separator: " ").lowercased() }
            func candidateResolves(_ match: NSTextCheckingResult) -> Bool {
                guard normalize(ns.substring(with: match.range(at: 1))) == normalize(label) else { return false }
                let line = ns.lineRange(for: match.range)
                let candidate = ns.substring(with: line) + "\n\n![probe][" + ns.substring(with: match.range(at: 1)) + "]"
                let parsed = Document(parsing: candidate)
                func containsImage(_ node: Markup) -> Bool {
                    if let value = node as? Markdown.Image { return value.source == image.destination }
                    return node.children.contains { containsImage($0) }
                }
                return containsImage(parsed)
            }
            guard let match = definitions.first(where: candidateResolves) else { return image }
            var destination = match.range(at: 2)
            if ns.substring(with: destination).hasPrefix("<") { destination.location += 1; destination.length -= 2 }
            return Reference(range: image.range, label: image.label, destination: image.destination, destinationRange: destination, title: image.title)
        }
    }
    public func headingOffset(for anchor: String, in source: String) -> Int? {
        var occurrences: [String: Int] = [:]
        let ns = source as NSString
        for heading in headings {
            let text = MarkdownAnalysis(ns.substring(with: heading.inner)).readableText
            let allowed = CharacterSet.alphanumerics.union(.whitespaces).union(CharacterSet(charactersIn: "_-"))
            let base = text.lowercased().unicodeScalars.filter { allowed.contains($0) }.map(String.init).joined().replacingOccurrences(of: " ", with: "-")
            let occurrence = occurrences[base, default: 0]; occurrences[base] = occurrence + 1
            let slug = base + (occurrence == 0 ? "" : "-\(occurrence)")
            if slug == (anchor.removingPercentEncoding ?? anchor) { return heading.range.location }
        }
        return nil
    }
    private static func inlineDestination(in text: NSString, range: NSRange) -> NSRange? {
        var i = range.location, bracketDepth = 0
        let end = NSMaxRange(range)
        while i < end {
            let c = text.character(at: i)
            if c == 92 { i += 2; continue }
            if c == 91 { bracketDepth += 1 }
            if c == 93 { bracketDepth -= 1; if bracketDepth == 0 { break } }
            i += 1
        }
        guard i + 1 < end, text.character(at: i + 1) == 40 else { return nil }
        i += 2
        while i < end, [UInt16(9), 10, 13, 32].contains(text.character(at: i)) { i += 1 }
        if i < end, text.character(at: i) == 60 {
            let start = i + 1; i += 1
            while i < end { if text.character(at: i) == 92 { i += 2; continue }; if text.character(at: i) == 62 { return NSRange(location: start, length: i - start) }; i += 1 }
            return nil
        }
        let start = i; var depth = 0
        while i < end {
            let c = text.character(at: i)
            if c == 92 { i += 2; continue }
            if c == 40 { depth += 1 }
            if c == 41 { if depth == 0 { break }; depth -= 1 }
            if c == 32 || c == 9 || c == 10 || c == 13 { break }
            i += 1
        }
        return NSRange(location: start, length: i - start)
    }
}

private struct SourceOffsets {
    let lines: [String]
    let starts: [Int]
    let length: Int
    init(_ source: String) {
        let ns = source as NSString
        var rows: [String] = [], positions: [Int] = [], start = 0, offset = 0
        // CommonMark counts only CR, LF and CRLF; NSString.lineRange also splits
        // Unicode paragraph separators and would shift every subsequent AST range.
        while offset < ns.length {
            let c = ns.character(at: offset)
            if c == 10 || c == 13 {
                offset += 1
                if c == 13, offset < ns.length, ns.character(at: offset) == 10 { offset += 1 }
                rows.append(ns.substring(with: NSRange(location: start, length: offset - start)))
                positions.append(start); start = offset
            } else { offset += 1 }
        }
        if start < ns.length { rows.append(ns.substring(from: start)); positions.append(start) }
        rows.append(""); positions.append(ns.length)
        lines = rows; starts = positions; length = ns.length
    }
    func range(_ source: SourceRange?) -> NSRange? {
        guard let source else { return nil }
        func offset(_ value: SourceLocation) -> Int {
            let row = min(max(0, value.line - 1), lines.count - 1)
            let bytes = Array(lines[row].utf8)
            let count = min(bytes.count, max(0, value.column - 1))
            return min(length, starts[row] + String(decoding: bytes.prefix(count), as: UTF8.self).utf16.count)
        }
        let start = offset(source.lowerBound), end = offset(source.upperBound)
        return NSRange(location: start, length: max(0, end - start))
    }
}
