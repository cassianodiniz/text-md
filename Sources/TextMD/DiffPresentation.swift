import AppKit
import TextMDCore

@MainActor
struct DeletedTextBlock {
    let anchor: Int
    let attributed: NSAttributedString
    let height: CGFloat
    init(_ deletion: DiffDeletion, width: CGFloat, zoom: CGFloat) {
        anchor = deletion.anchor
        var text = deletion.text
        if text.hasSuffix("\r\n") { text.removeLast() }
        else if text.hasSuffix("\n") || text.hasSuffix("\r") { text.removeLast() }
        let paragraph = NSMutableParagraphStyle()
        paragraph.lineSpacing = 5 * zoom; paragraph.lineBreakMode = .byWordWrapping
        attributed = NSAttributedString(string: text.isEmpty ? " " : text, attributes: [
            .font: NSFont.monospacedSystemFont(ofSize: 14 * zoom, weight: .regular),
            .foregroundColor: NSColor.systemRed, .paragraphStyle: paragraph,
            .strikethroughStyle: NSUnderlineStyle.single.rawValue
        ])
        let rect = attributed.boundingRect(with: NSSize(width: max(100, width - 20), height: .greatestFiniteMagnitude),
                                           options: [.usesLineFragmentOrigin, .usesFontLeading])
        height = ceil(rect.height) + 12 * zoom
    }
}

@MainActor
enum DiffPresentation {
    static func apply(_ diff: InlineDiff, storage: NSTextStorage, layout: MarkdownLayoutManager, width: CGFloat, zoom: CGFloat) -> CGFloat {
        layout.deletedBlocks = diff.deletions.map { DeletedTextBlock($0, width: width, zoom: zoom) }
        for range in diff.additions where range.length > 0 {
            storage.addAttributes([.foregroundColor: NSColor.systemGreen, .backgroundColor: NSColor.systemGreen.withAlphaComponent(0.12)], range: range)
        }
        var top: CGFloat = 0
        for block in layout.deletedBlocks where block.anchor < storage.length {
            if block.anchor == 0 { top += block.height + 8 * zoom; continue }
            let range = (storage.string as NSString).paragraphRange(for: NSRange(location: block.anchor, length: 0))
            let paragraph = (storage.attribute(.paragraphStyle, at: block.anchor, effectiveRange: nil) as? NSParagraphStyle)?.mutableCopy() as? NSMutableParagraphStyle ?? NSMutableParagraphStyle()
            paragraph.paragraphSpacingBefore += block.height + 8 * zoom
            storage.addAttribute(.paragraphStyle, value: paragraph, range: range)
        }
        layout.terminalDeletionHeight = layout.deletedBlocks.filter { $0.anchor == storage.length }.reduce(0) { $0 + $1.height + 8 * zoom }
        return top
    }
}

extension MarkdownLayoutManager {
    func drawDeletions(at origin: NSPoint, dirtyRect: NSRect) {
        guard let storage = textStorage, let container = textContainers.first else { return }
        for block in deletedBlocks {
            let y: CGFloat
            if block.anchor < storage.length {
                let glyph = glyphIndexForCharacter(at: block.anchor)
                // Line-fragment rects include paragraphSpacingBefore. The glyph
                // rect starts after that reserved gap; using the fragment would
                // paint the deleted text over the previous unchanged paragraph.
                y = boundingRect(forGlyphRange: NSRange(location: glyph, length: 1), in: container).minY - block.height - 8 * zoom
            } else if storage.length == 0 { y = max(22, extraLineFragmentRect.maxY) + 8 }
            else if (storage.string as NSString).hasSuffix("\n") || (storage.string as NSString).hasSuffix("\r") {
                y = extraLineFragmentRect.maxY + 8 * zoom
            } else {
                let last = glyphIndexForCharacter(at: storage.length - 1)
                y = lineFragmentRect(forGlyphAt: last, effectiveRange: nil).maxY + 8 * zoom
            }
            let rect = NSRect(x: origin.x - 5, y: origin.y + y, width: container.size.width + 10, height: block.height)
            guard rect.intersects(dirtyRect) else { continue }
            NSColor.systemRed.withAlphaComponent(0.09).setFill()
            NSBezierPath(roundedRect: rect, xRadius: 4, yRadius: 4).fill()
            block.attributed.draw(with: rect.insetBy(dx: 10, dy: 6), options: [.usesLineFragmentOrigin, .usesFontLeading])
            ("−" as NSString).draw(at: NSPoint(x: rect.minX - 17, y: rect.minY + 6), withAttributes: [
                .font: NSFont.monospacedSystemFont(ofSize: 14 * zoom, weight: .medium), .foregroundColor: NSColor.systemRed
            ])
        }
    }
}
