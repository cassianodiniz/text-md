import AppKit
import TextMDCore

extension NSAttributedString.Key {
    static let imagePreview = NSAttributedString.Key("textmd.imagePreview")
    static let imageHeight = NSAttributedString.Key("textmd.imageHeight")
    static let tableRow = NSAttributedString.Key("textmd.tableRow")
    static let tablePreview = NSAttributedString.Key("textmd.tablePreview")
    static let frontMatter = NSAttributedString.Key("textmd.frontMatter")
    static let headingUnderline = NSAttributedString.Key("textmd.headingUnderline")
    static let quoteBorder = NSAttributedString.Key("textmd.quoteBorder")
    static let horizontalRule = NSAttributedString.Key("textmd.horizontalRule")
    static let taskBox = NSAttributedString.Key("textmd.taskBox")
}

final class TablePreview: NSObject {
    let cells: [String]
    let header: Bool
    init(cells: [String], header: Bool) { self.cells = cells; self.header = header }
}

/// The storage always contains the original Markdown. Formatting only changes
/// attributes, so opening documents, copying and mode switches are lossless.
@MainActor
enum MarkdownStyler {
    static let bodySize: CGFloat = 19
    static func serif(_ size: CGFloat) -> NSFont {
        NSFont(name: "Vollkorn", size: size) ?? NSFont(name: "Palatino-Roman", size: size) ?? NSFont(name: "TimesNewRomanPSMT", size: size) ?? NSFont.systemFont(ofSize: size)
    }
    static var paperColor: NSColor {
        NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua ? NSColor(calibratedWhite: 0.12, alpha: 1) : NSColor(calibratedWhite: 254.0 / 255, alpha: 1)
        }
    }
    static var textColor: NSColor {
        NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua ? NSColor(calibratedWhite: 0.88, alpha: 1) : NSColor(calibratedWhite: 0.2, alpha: 1)
        }
    }
    static var linkColor: NSColor {
        NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua ? NSColor(calibratedRed: 0.43, green: 0.72, blue: 0.91, alpha: 1) : NSColor(calibratedRed: 36.0 / 255, green: 132.0 / 255, blue: 193.0 / 255, alpha: 1)
        }
    }
    static var metaBackgroundColor: NSColor {
        NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua ? NSColor(calibratedWhite: 0.16, alpha: 1) : NSColor(calibratedWhite: 248.0 / 255, alpha: 1)
        }
    }
    static func style(_ storage: NSTextStorage, activeRange: NSRange, source: Bool,
                      width: CGFloat, documentPath: String?, assetsRoot: URL?, zoom: CGFloat = 1, metaOffset: CGFloat = 0) {
        let text = storage.string as NSString
        let whole = NSRange(location: 0, length: text.length)
        let paragraph = NSMutableParagraphStyle()
        paragraph.lineSpacing = source ? 5 : 0
        paragraph.paragraphSpacing = source ? 0 : 19
        if !source { paragraph.minimumLineHeight = bodySize * 1.53; paragraph.alignment = .justified }
        storage.beginEditing()
        storage.setAttributes([
            .font: source ? NSFont.monospacedSystemFont(ofSize: 14, weight: .regular) : serif(bodySize),
            .foregroundColor: textColor,
            .paragraphStyle: paragraph
        ], range: whole)
        defer {
            if zoom != 1 { scaleAttributes(storage, by: zoom) }
            storage.endEditing()
        }
        guard !source, text.length > 0 else { return }

        let active: NSRange? = activeRange.location == NSNotFound ? nil : text.paragraphRange(for: NSRange(location: min(activeRange.location, text.length), length: 0))
        func isActive(_ range: NSRange) -> Bool {
            guard let active else { return false }
            return NSIntersectionRange(range, active).length > 0
        }
        func mark(_ range: NSRange, parent: NSRange) {
            guard range.length > 0 else { return }
            if isActive(parent) {
                storage.addAttribute(.foregroundColor, value: NSColor.tertiaryLabelColor, range: range)
            } else {
                storage.addAttributes([.font: NSFont.systemFont(ofSize: 0.1), .foregroundColor: NSColor.clear, .spellingState: 0], range: range)
            }
        }
        var inFence = false
        var fenceMarker = ""
        var inFrontMatter = false
        var position = 0
        var previousHeading = 0
        var previousSpacing: CGFloat = 0
        while position < text.length {
            let range = text.lineRange(for: NSRange(location: position, length: 0))
            let raw = text.substring(with: range)
            let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
            let contentLength = (raw.trimmingCharacters(in: .newlines) as NSString).length
            let contentRange = NSRange(location: range.location, length: contentLength)
            defer { position = NSMaxRange(range) }
            if trimmed.isEmpty && !isActive(range) && !inFence && !inFrontMatter {
                let compact = NSMutableParagraphStyle(); compact.minimumLineHeight = 0.1; compact.maximumLineHeight = 0.1
                storage.addAttributes([.font: NSFont.systemFont(ofSize: 0.1), .paragraphStyle: compact], range: range)
                continue
            }
            if position == 0, trimmed == "---" { inFrontMatter = true }
            if inFrontMatter {
                let delimiter = trimmed == "---" || trimmed == "..."
                let style = NSMutableParagraphStyle()
                style.minimumLineHeight = 19 * 0.76 * 1.8
                style.paragraphSpacing = 0
                style.lineBreakMode = .byClipping
                style.firstLineHeadIndent = -metaOffset / zoom
                style.headIndent = -metaOffset / zoom
                if position == 0 { previousSpacing = 0 }
                if delimiter && !isActive(range) { style.minimumLineHeight = 1; style.maximumLineHeight = 1 }
                if position > 0 && delimiter { style.paragraphSpacing = 38; previousSpacing = 38; inFrontMatter = false }
                storage.addAttributes([.font: NSFont.monospacedSystemFont(ofSize: 19 * 0.76, weight: .regular),
                    .foregroundColor: NSColor(calibratedWhite: 0.6, alpha: 1), .paragraphStyle: style], range: range)
                if !delimiter {
                    storage.addAttributes([.frontMatter: true, .foregroundColor: NSColor.clear, .spellingState: 0], range: range)
                }
                else { mark(contentRange, parent: range) }
                continue
            }
            if trimmed.hasPrefix("```") || trimmed.hasPrefix("~~~") {
                let token = String(trimmed.prefix(3))
                if !inFence { inFence = true; fenceMarker = token }
                else if token == fenceMarker { inFence = false }
                storage.addAttributes([.font: NSFont.monospacedSystemFont(ofSize: 14, weight: .regular),
                    .foregroundColor: NSColor.tertiaryLabelColor,
                    .backgroundColor: NSColor.quaternaryLabelColor.withAlphaComponent(0.15)], range: range)
                if !isActive(range) { mark(contentRange, parent: range) }
                continue
            }
            if inFence || raw.hasPrefix("    ") || raw.hasPrefix("\t") {
                storage.addAttributes([.font: NSFont.monospacedSystemFont(ofSize: 14, weight: .regular),
                    .backgroundColor: NSColor.quaternaryLabelColor.withAlphaComponent(0.15)], range: range)
                continue
            }
            if let match = match("^(#{1,6})[ \\t]+", in: raw) {
                let level = match.range(at: 1).length
                let sizes: [CGFloat] = [57, 34.2, 30.4, 26.6, 22.8, 19]
                var font = serif(sizes[level - 1])
                if level == 3 { font = NSFontManager.shared.convert(font, toHaveTrait: .italicFontMask) }
                if level >= 4 { font = NSFontManager.shared.convert(font, toHaveTrait: .boldFontMask) }
                let style = paragraph.mutableCopy() as! NSMutableParagraphStyle
                style.alignment = level <= 3 ? .center : .left
                style.minimumLineHeight = sizes[level - 1] * 1.53
                style.paragraphSpacingBefore = level == 1 ? 57 * 1.6 : level == 2 ? 34.2 * 2 : level == 3 ? 30.4 * 3 : 19
                if previousHeading == level - 1 && (level == 2 || level == 3) { style.paragraphSpacingBefore = sizes[level - 1] * 0.83 }
                // CSS adjoining margins collapse rather than accumulate. AppKit
                // paragraph spacing is additive, so subtract the preceding gap.
                style.paragraphSpacingBefore = max(0, style.paragraphSpacingBefore - previousSpacing)
                style.paragraphSpacing = level == 2 ? 28 : 19
                previousSpacing = style.paragraphSpacing
                storage.addAttributes([.font: font, .paragraphStyle: style], range: range)
                if level == 2 { storage.addAttribute(.headingUnderline, value: true, range: contentRange) }
                previousHeading = level
                mark(NSRange(location: range.location, length: match.range.length), parent: range)
            } else { previousSpacing = 19; previousHeading = 0 }
            if let match = match("^([ \\t]*>[ \\t]?)", in: raw) {
                storage.addAttribute(.foregroundColor, value: NSColor.secondaryLabelColor, range: range)
                let style = paragraph.mutableCopy() as! NSMutableParagraphStyle
                style.headIndent = 38; style.firstLineHeadIndent = 38
                storage.addAttribute(.quoteBorder, value: true, range: contentRange)
                storage.addAttribute(.paragraphStyle, value: style, range: range)
                mark(NSRange(location: range.location, length: match.range.length), parent: range)
            }
            if let task = match("^[ \\t]*[-*+] \\[([ xX])\\] ", in: raw) {
                let boxRange = NSRange(location: range.location + task.range(at: 1).location - 1, length: 3)
                if !isActive(range) {
                    mark(NSRange(location: range.location, length: task.range.length), parent: range)
                    // Reserve space for a native checkbox without replacing the source.
                    let style = paragraph.mutableCopy() as! NSMutableParagraphStyle
                    style.firstLineHeadIndent = 26; style.headIndent = 26
                    storage.addAttribute(.paragraphStyle, value: style, range: range)
                    storage.addAttribute(.taskBox, value: text.substring(with: boxRange), range: contentRange)
                }
            } else if let list = match("^[ \\t]*([-*+] |[0-9]+[.)] )", in: raw) {
                storage.addAttribute(.foregroundColor, value: NSColor.secondaryLabelColor,
                    range: NSRange(location: range.location, length: list.range.length))
            }
            if trimmed.hasPrefix("|") && trimmed.contains("|") {
                let separator = trimmed.range(of: "^\\|?[ :|-]+\\|[ :|-]*$", options: .regularExpression) != nil
                let tableStyle = NSMutableParagraphStyle()
                tableStyle.lineSpacing = 0; tableStyle.paragraphSpacing = 0
                tableStyle.minimumLineHeight = separator && !isActive(range) ? 1 : 36
                tableStyle.maximumLineHeight = separator && !isActive(range) ? 1 : 36
                storage.addAttributes([.paragraphStyle: tableStyle, .font: NSFont.monospacedSystemFont(ofSize: 15, weight: .regular)], range: range)
                if !isActive(range) {
                    mark(contentRange, parent: range)
                    if !separator {
                        let next = NSMaxRange(range) < text.length ? text.substring(with: text.lineRange(for: NSRange(location: NSMaxRange(range), length: 0))).trimmingCharacters(in: .whitespacesAndNewlines) : ""
                        let header = next.range(of: "^\\|?[ :|-]+\\|[ :|-]*$", options: .regularExpression) != nil
                        var cells = trimmed.replacingOccurrences(of: "\\|", with: "\u{E000}").components(separatedBy: "|")
                        if cells.first?.isEmpty == true { cells.removeFirst() }
                        if cells.last?.isEmpty == true { cells.removeLast() }
                        cells = cells.map { $0.trimmingCharacters(in: .whitespaces).replacingOccurrences(of: "\u{E000}", with: "|") }
                        storage.addAttribute(.tablePreview, value: TablePreview(cells: cells, header: header), range: contentRange)
                    }
                }
                continue
            }
            if ["---", "***", "___"].contains(trimmed) {
                storage.addAttribute(.horizontalRule, value: true, range: contentRange)
                mark(contentRange, parent: range)
            }
            if let image = match("!\\[([^\\]]*)\\]\\(([^)]+)\\)", in: raw),
               let root = assetsRoot {
                let ref = (raw as NSString).substring(with: image.range(at: 2))
                if let url = imageURL(ref, documentPath: documentPath, assetsRoot: root),
                   let preview = NSImage(contentsOf: url), preview.size.width > 0 {
                    let targetWidth = min(max(120, width - 30), preview.size.width)
                    let height = min(420, targetWidth * preview.size.height / preview.size.width)
                    let style = paragraph.mutableCopy() as! NSMutableParagraphStyle
                    style.paragraphSpacingBefore = height + 18
                    storage.addAttributes([.paragraphStyle: style, .imagePreview: preview, .imageHeight: height], range: contentRange)
                    if !isActive(range) { mark(contentRange, parent: range) }
                }
                continue
            }
            for (pattern, trait) in [("(\\*\\*|__)(.+?)\\1", NSFontTraitMask.boldFontMask),
                                      ("(?<!\\*)(\\*)([^*\\n]+)\\1(?!\\*)", .italicFontMask),
                                      ("(?<!_)(_)([^_\\n]+)\\1(?!_)", .italicFontMask)] {
                guard let regex = try? NSRegularExpression(pattern: pattern) else { continue }
                for match in regex.matches(in: raw, range: NSRange(location: 0, length: (raw as NSString).length)) {
                    let start = range.location + match.range.location
                    let inner = NSRange(location: range.location + match.range(at: 2).location, length: match.range(at: 2).length)
                    let font = storage.attribute(.font, at: inner.location, effectiveRange: nil) as? NSFont ?? serif(bodySize)
                    storage.addAttribute(.font, value: NSFontManager.shared.convert(font, toHaveTrait: trait), range: inner)
                    mark(NSRange(location: start, length: match.range(at: 1).length), parent: range)
                    mark(NSRange(location: range.location + NSMaxRange(match.range) - match.range(at: 1).length,
                                 length: match.range(at: 1).length), parent: range)
                }
            }
            if let regex = try? NSRegularExpression(pattern: "`([^`\\n]+)`") {
                for code in regex.matches(in: raw, range: NSRange(location: 0, length: (raw as NSString).length)) {
                    let inner = NSRange(location: range.location + code.range(at: 1).location, length: code.range(at: 1).length)
                    storage.addAttributes([.font: NSFont.monospacedSystemFont(ofSize: 16, weight: .regular),
                        .backgroundColor: NSColor.quaternaryLabelColor.withAlphaComponent(0.12)], range: inner)
                    mark(NSRange(location: range.location + code.range.location, length: 1), parent: range)
                    mark(NSRange(location: range.location + NSMaxRange(code.range) - 1, length: 1), parent: range)
                }
            }
            for link in linkRanges(raw) {
                let label = NSRange(location: range.location + link.label.location, length: link.label.length)
                let destination = (raw as NSString).substring(with: link.destination)
                storage.addAttributes([.foregroundColor: linkColor, .link: destination], range: label)
                mark(NSRange(location: range.location + link.whole.location, length: label.location - range.location - link.whole.location), parent: range)
                mark(NSRange(location: NSMaxRange(label), length: range.location + NSMaxRange(link.whole) - NSMaxRange(label)), parent: range)
                if isActive(range) {
                    storage.addAttributes([.foregroundColor: linkColor.withAlphaComponent(0.6), .underlineStyle: NSUnderlineStyle.single.rawValue],
                        range: NSRange(location: range.location + link.destination.location, length: link.destination.length))
                }
            }
            if let regex = try? NSRegularExpression(pattern: "~~(.+?)~~") {
                for strike in regex.matches(in: raw, range: NSRange(location: 0, length: (raw as NSString).length)) {
                    let inner = NSRange(location: range.location + strike.range(at: 1).location, length: strike.range(at: 1).length)
                    storage.addAttribute(.strikethroughStyle, value: NSUnderlineStyle.single.rawValue, range: inner)
                    mark(NSRange(location: range.location + strike.range.location, length: 2), parent: range)
                    mark(NSRange(location: range.location + NSMaxRange(strike.range) - 2, length: 2), parent: range)
                }
            }
        }
    }
    private static func scaleAttributes(_ storage: NSTextStorage, by zoom: CGFloat) {
        let range = NSRange(location: 0, length: storage.length)
        var changes: [(NSRange, [NSAttributedString.Key: Any])] = []
        storage.enumerateAttributes(in: range) { attributes, range, _ in
            var scaled: [NSAttributedString.Key: Any] = [:]
            if let font = attributes[.font] as? NSFont { scaled[.font] = NSFontManager.shared.convert(font, toSize: font.pointSize * zoom) }
            if let original = attributes[.paragraphStyle] as? NSParagraphStyle, let style = original.mutableCopy() as? NSMutableParagraphStyle {
                style.lineSpacing *= zoom; style.paragraphSpacing *= zoom; style.paragraphSpacingBefore *= zoom
                style.minimumLineHeight *= zoom; style.maximumLineHeight *= zoom
                style.firstLineHeadIndent *= zoom; style.headIndent *= zoom; style.tailIndent *= zoom
                scaled[.paragraphStyle] = style
            }
            if let height = attributes[.imageHeight] as? CGFloat { scaled[.imageHeight] = height * zoom }
            changes.append((range, scaled))
        }
        for (range, values) in changes { storage.addAttributes(values, range: range) }
    }
    private struct LinkRanges { let whole: NSRange, label: NSRange, destination: NSRange }
    private static func linkRanges(_ value: String) -> [LinkRanges] {
        let ns = value as NSString
        guard let regex = try? NSRegularExpression(pattern: "(?<!!|\\\\)\\[([^\\]]+)\\]\\(") else { return [] }
        return regex.matches(in: value, range: NSRange(location: 0, length: ns.length)).compactMap { match in
            var i = NSMaxRange(match.range), depth = 1
            let start = i
            while i < ns.length && depth > 0 {
                let c = ns.character(at: i)
                if c == 92 { i += 2; continue }
                if c == 40 { depth += 1 }; if c == 41 { depth -= 1 }; i += 1
            }
            guard depth == 0 else { return nil }
            return LinkRanges(whole: NSRange(location: match.range.location, length: i - match.range.location),
                label: match.range(at: 1), destination: NSRange(location: start, length: i - start - 1))
        }
    }
    private static func match(_ pattern: String, in text: String) -> NSTextCheckingResult? {
        (try? NSRegularExpression(pattern: pattern))?.firstMatch(in: text, range: NSRange(location: 0, length: (text as NSString).length))
    }
    static func imageURL(_ raw: String, documentPath: String?, assetsRoot: URL) -> URL? {
        let ref = raw.trimmingCharacters(in: .whitespaces).trimmingCharacters(in: CharacterSet(charactersIn: "<>"))
        if ref.hasPrefix("textmd-asset:") { return try? FileService.assetURL(ref, assetsRoot: assetsRoot) }
        if ref.hasPrefix("file://") {
            guard let url = URL(string: ref), url.host == nil || url.host?.isEmpty == true || url.host == "localhost" else { return nil }
            return url
        }
        // Local app: external URLs remain editable references and aren't fetched silently.
        if ref.contains("://") { return nil }
        let decoded = ref.removingPercentEncoding ?? ref
        if decoded.hasPrefix("/") { return URL(fileURLWithPath: decoded) }
        guard let documentPath else { return nil }
        return URL(fileURLWithPath: documentPath).deletingLastPathComponent().appendingPathComponent(decoded)
    }
}

@MainActor
final class MarkdownLayoutManager: NSLayoutManager {
    var deletedBlocks: [DeletedTextBlock] = []
    var terminalDeletionHeight: CGFloat = 0
    var zoom: CGFloat = 1
    var metaOffset: CGFloat = 0
    override func drawGlyphs(forGlyphRange glyphsToShow: NSRange, at origin: NSPoint) {
        super.drawGlyphs(forGlyphRange: glyphsToShow, at: origin)
        guard let storage = textStorage, let container = textContainers.first else { return }
        let visible = characterRange(forGlyphRange: glyphsToShow, actualGlyphRange: nil)
        storage.enumerateAttribute(.frontMatter, in: visible) { value, range, _ in
            guard value != nil else { return }
            let text = storage.string as NSString
            var position = range.location
            while position < NSMaxRange(range) {
                let line = text.lineRange(for: NSRange(location: position, length: 0))
                let glyph = self.glyphIndexForCharacter(at: position)
                let rect = self.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
                let clip = NSRect(x: origin.x, y: origin.y + rect.minY, width: container.size.width, height: rect.height)
                NSGraphicsContext.saveGraphicsState(); NSBezierPath(rect: clip).addClip()
                let paragraph = NSMutableParagraphStyle(); paragraph.lineBreakMode = .byClipping
                let attrs: [NSAttributedString.Key: Any] = [.font: NSFont.monospacedSystemFont(ofSize: 19 * 0.76 * self.zoom, weight: .regular),
                    .foregroundColor: NSColor.secondaryLabelColor, .paragraphStyle: paragraph]
                let raw = text.substring(with: line).trimmingCharacters(in: .newlines)
                let width = (raw as NSString).size(withAttributes: attrs).width + 10
                (raw as NSString).draw(in: NSRect(x: origin.x + 5 - self.metaOffset, y: origin.y + rect.minY,
                    width: max(width, container.size.width + self.metaOffset), height: rect.height), withAttributes: attrs)
                NSGraphicsContext.restoreGraphicsState()
                position = NSMaxRange(line)
            }
        }
    }
    override func drawBackground(forGlyphRange glyphsToShow: NSRange, at origin: NSPoint) {
        super.drawBackground(forGlyphRange: glyphsToShow, at: origin)
        guard let storage = textStorage, let container = textContainers.first, storage.length > 0 else { return }
        let chars = characterRange(forGlyphRange: glyphsToShow, actualGlyphRange: nil)
        storage.enumerateAttribute(.frontMatter, in: chars) { value, range, _ in
            guard value != nil else { return }
            let glyphs = self.glyphRange(forCharacterRange: range, actualCharacterRange: nil)
            self.enumerateLineFragments(forGlyphRange: glyphs) { rect, _, _, _, _ in
                MarkdownStyler.metaBackgroundColor.setFill()
                NSRect(x: origin.x - 30, y: origin.y + rect.minY, width: container.size.width + 60, height: rect.height).fill()
            }
        }
        storage.enumerateAttribute(.headingUnderline, in: chars) { value, range, _ in
            guard value != nil else { return }
            let glyphs = self.glyphRange(forCharacterRange: range, actualCharacterRange: nil)
            let rect = self.boundingRect(forGlyphRange: glyphs, in: container)
            MarkdownStyler.textColor.setFill()
            NSRect(x: origin.x + container.size.width / 2 - 50 * self.zoom, y: origin.y + rect.maxY + 7 * self.zoom, width: 100 * self.zoom, height: self.zoom).fill()
        }
        storage.enumerateAttribute(.quoteBorder, in: chars) { value, range, _ in
            guard value != nil else { return }
            let glyphs = self.glyphRange(forCharacterRange: range, actualCharacterRange: nil)
            let rect = self.boundingRect(forGlyphRange: glyphs, in: container)
            NSColor.separatorColor.setFill()
            NSRect(x: origin.x + 19 * self.zoom, y: origin.y + rect.minY, width: 1, height: rect.height).fill()
        }
        storage.enumerateAttribute(.horizontalRule, in: chars) { value, range, _ in
            guard value != nil else { return }
            let glyph = self.glyphIndexForCharacter(at: range.location)
            let rect = self.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
            NSColor.separatorColor.setFill()
            NSRect(x: origin.x, y: origin.y + rect.midY, width: container.size.width, height: 1).fill()
        }
        storage.enumerateAttribute(.imagePreview, in: chars) { value, range, _ in
            guard let image = value as? NSImage else { return }
            let glyph = self.glyphRange(forCharacterRange: NSRange(location: range.location, length: 1), actualCharacterRange: nil)
            let rect = self.boundingRect(forGlyphRange: glyph, in: container)
            let height = storage.attribute(.imageHeight, at: range.location, effectiveRange: nil) as? CGFloat ?? 200
            let width = min(container.size.width - 30, height * image.size.width / max(1, image.size.height))
            let target = NSRect(x: origin.x + rect.minX, y: origin.y + rect.minY - height - 10, width: width, height: height)
            image.draw(in: target, from: .zero, operation: .sourceOver, fraction: 1, respectFlipped: true, hints: nil)
        }
        storage.enumerateAttribute(.taskBox, in: chars) { value, range, _ in
            guard let box = value as? String else { return }
            let glyph = self.glyphRange(forCharacterRange: NSRange(location: range.location, length: 1), actualCharacterRange: nil)
            let rect = self.boundingRect(forGlyphRange: glyph, in: container)
            let target = NSRect(x: origin.x + 2 * self.zoom, y: origin.y + rect.minY + 4 * self.zoom, width: 15 * self.zoom, height: 15 * self.zoom)
            let path = NSBezierPath(roundedRect: target, xRadius: 3 * self.zoom, yRadius: 3 * self.zoom)
            NSColor.secondaryLabelColor.setStroke(); path.lineWidth = 1.2 * self.zoom; path.stroke()
            if box.lowercased().contains("x") {
                let check = NSBezierPath(); check.move(to: NSPoint(x: target.minX + 3 * self.zoom, y: target.midY))
                check.line(to: NSPoint(x: target.midX - self.zoom, y: target.maxY - 3 * self.zoom)); check.line(to: NSPoint(x: target.maxX - 3 * self.zoom, y: target.minY + 3 * self.zoom))
                check.lineWidth = 1.7 * self.zoom; NSColor.controlAccentColor.setStroke(); check.stroke()
            }
        }
        storage.enumerateAttribute(.tablePreview, in: chars) { value, range, _ in
            guard let table = value as? TablePreview, !table.cells.isEmpty else { return }
            let glyph = self.glyphIndexForCharacter(at: range.location)
            let line = self.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
            let rect = NSRect(x: origin.x + 4, y: origin.y + line.minY, width: container.size.width - 8, height: 36 * self.zoom)
            if table.header { NSColor.quaternaryLabelColor.withAlphaComponent(0.15).setFill(); rect.fill() }
            NSColor.separatorColor.setStroke()
            let border = NSBezierPath(rect: rect); border.lineWidth = 0.6; border.stroke()
            let cellWidth = rect.width / CGFloat(table.cells.count)
            for (index, cell) in table.cells.enumerated() {
                let cellRect = NSRect(x: rect.minX + CGFloat(index) * cellWidth + 10 * self.zoom, y: rect.minY + 8 * self.zoom,
                                      width: cellWidth - 20 * self.zoom, height: 24 * self.zoom)
                let font = table.header ? NSFontManager.shared.convert(MarkdownStyler.serif(16 * self.zoom), toHaveTrait: .boldFontMask) : MarkdownStyler.serif(16 * self.zoom)
                let p = NSMutableParagraphStyle(); p.lineBreakMode = .byTruncatingTail
                let string = NSAttributedString(string: cell, attributes: [.font: font, .foregroundColor: NSColor.labelColor, .paragraphStyle: p])
                string.draw(with: cellRect, options: [.usesLineFragmentOrigin, .truncatesLastVisibleLine])
                if index > 0 {
                    let x = rect.minX + CGFloat(index) * cellWidth
                    let divider = NSBezierPath(); divider.move(to: NSPoint(x: x, y: rect.minY)); divider.line(to: NSPoint(x: x, y: rect.maxY))
                    divider.lineWidth = 0.6; divider.stroke()
                }
            }
        }
    }
}
