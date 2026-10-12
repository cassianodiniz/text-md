import AppKit
import SwiftUI
import TextMDCore

enum MarkdownAction { case bold, italic, heading, link, bullet, task, code, table }

struct MarkdownEditor: NSViewRepresentable {
    let draft: Draft
    let model: AppModel
    func makeNSView(context: Context) -> NSScrollView {
        let storage = NSTextStorage()
        let layout = MarkdownLayoutManager()
        let container = NSTextContainer(containerSize: NSSize(width: 800, height: CGFloat.greatestFiniteMagnitude))
        storage.addLayoutManager(layout); layout.addTextContainer(container)
        let editor = MarkdownTextView(frame: NSRect(x: 0, y: 0, width: 800, height: 600), textContainer: container)
        editor.model = model; editor.delegate = editor
        editor.isRichText = false; editor.importsGraphics = false; editor.allowsUndo = true
        editor.isAutomaticQuoteSubstitutionEnabled = false
        editor.isAutomaticDashSubstitutionEnabled = false
        editor.isAutomaticTextReplacementEnabled = false
        editor.isAutomaticSpellingCorrectionEnabled = false
        editor.isContinuousSpellCheckingEnabled = true
        editor.linkTextAttributes = [.foregroundColor: MarkdownStyler.linkColor, .underlineStyle: 0]
        editor.usesFindBar = true
        editor.isIncrementalSearchingEnabled = true
        editor.isVerticallyResizable = true; editor.isHorizontallyResizable = false
        editor.minSize = .zero; editor.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        editor.autoresizingMask = [.width]
        editor.backgroundColor = MarkdownStyler.paperColor
        editor.setAccessibilityIdentifier("markdown-editor")
        editor.setAccessibilityLabel("Editor Markdown")
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 800, height: 600))
        scroll.hasVerticalScroller = true; scroll.hasHorizontalScroller = false
        scroll.autohidesScrollers = true; scroll.borderType = .noBorder
        scroll.backgroundColor = MarkdownStyler.paperColor
        scroll.documentView = editor
        scroll.contentView.postsBoundsChangedNotifications = true
        editor.boundsObserver = NotificationCenter.default.addObserver(forName: NSView.boundsDidChangeNotification,
            object: scroll.contentView, queue: .main) { [weak editor] _ in
                MainActor.assumeIsolated { editor?.positionChanged() }
            }
        scroll.verticalRulerView = LineNumberRuler(textView: editor)
        scroll.hasVerticalRuler = true
        editor.configure(draft: draft, source: model.session.sourceMode, lineNumbers: model.session.showLineNumbers, diff: model.diffEnabled, original: model.originalForDiff, zoom: model.zoom)
        model.editor = editor
        return scroll
    }
    func updateNSView(_ scroll: NSScrollView, context: Context) {
        guard let editor = scroll.documentView as? MarkdownTextView else { return }
        editor.configure(draft: draft, source: model.session.sourceMode, lineNumbers: model.session.showLineNumbers, diff: model.diffEnabled, original: model.originalForDiff, zoom: model.zoom)
        model.editor = editor
    }
}

@MainActor
final class MarkdownTextView: NSTextView, NSTextViewDelegate {
    weak var model: AppModel?
    var draftID: UUID?
    var documentPath: String?
    var sourceMode = false
    var boundsObserver: NSObjectProtocol?
    private var configuring = false
    private var styling = false
    private var lastActiveParagraph: NSRange?
    private var positionWork: DispatchWorkItem?
    private var storedContent = ""
    private var storedSelection = 0
    private var storedScroll = 0.0
    var lineStarts: [Int] = [0]
    private var showingDiff = false
    private var diffOriginal: String?
    private var measuringHeight = false
    private(set) var zoom: CGFloat = 1
    private var metaOffset: CGFloat = 0
    private var metaMaximumOffset: CGFloat = 0
    private lazy var metaScroller: NSScroller = {
        let control = NSScroller(frame: NSRect(x: 0, y: 0, width: 100, height: 12))
        control.scrollerStyle = .overlay; control.controlSize = .small
        control.target = self; control.action = #selector(scrollMeta(_:))
        control.setAccessibilityIdentifier("metadata-scrollbar")
        control.setAccessibilityLabel("Rolar propriedades horizontalmente")
        control.setAccessibilityElement(true)
        enclosingScrollView?.addSubview(control); return control
    }()

    func configure(draft: Draft, source: Bool, lineNumbers: Bool, diff: Bool, original: String?, zoom: CGFloat) {
        let changedDocument = draftID != draft.id
        let modeChanged = sourceMode != (source || diff) || showingDiff != diff || diffOriginal != original
        let pathChanged = documentPath != draft.sourcePath
        let zoomChanged = self.zoom != zoom
        self.zoom = zoom
        sourceMode = source || diff; showingDiff = diff; diffOriginal = original; documentPath = draft.sourcePath
        enclosingScrollView?.rulersVisible = source && lineNumbers && !diff
        guard changedDocument || string != draft.content || modeChanged || pathChanged || zoomChanged else { return }
        configuring = true
        if changedDocument || string != draft.content {
            if !changedDocument && string != draft.content { undoManager?.removeAllActions() }
            draftID = draft.id
            if changedDocument { metaOffset = 0 }
            string = draft.content
            setSelectedRange(NSRange(location: min(draft.selection, (string as NSString).length), length: 0))
            storedContent = string; storedSelection = selectedRange().location; storedScroll = draft.scrollPosition
            rebuildLines()
        }
        resizeContainer(); restyle()
        configuring = false
        if changedDocument {
            DispatchQueue.main.async { [weak self] in
                guard let self else { return }
                self.configuring = true
                if let clip = self.enclosingScrollView?.contentView {
                    let y = min(CGFloat(draft.scrollPosition), max(0, self.frame.height - clip.bounds.height))
                    clip.scroll(to: NSPoint(x: 0, y: y)); self.enclosingScrollView?.reflectScrolledClipView(clip)
                }
                self.configuring = false
                self.window?.makeFirstResponder(self)
            }
        }
    }
    override var undoManager: UndoManager? {
        if let id = draftID { return model?.undoManager(for: id) }
        return super.undoManager
    }
    override func setFrameSize(_ newSize: NSSize) {
        let oldWidth = frame.width
        var adjusted = newSize
        if !measuringHeight, let layout = layoutManager as? MarkdownLayoutManager, let container = textContainer, layout.terminalDeletionHeight > 0 {
            measuringHeight = true
            adjusted.height = max(adjusted.height, layout.usedRect(for: container).maxY + textContainerInset.height * 2 + layout.terminalDeletionHeight)
            measuringHeight = false
        }
        super.setFrameSize(adjusted)
        guard abs(oldWidth - newSize.width) > 1 else { return }
        resizeContainer()
        if !configuring { restyle() }
    }
    private func resizeContainer() {
        let available = max(200, frame.width)
        let textWidth = sourceMode ? available - 84 : min(available >= 1400 ? 1100 : 960, available - 60)
        textContainerInset = NSSize(width: max(30, (available - textWidth) / 2), height: 40 * zoom)
        textContainer?.widthTracksTextView = false
        textContainer?.containerSize = NSSize(width: textWidth, height: CGFloat.greatestFiniteMagnitude)
    }
    func restyle() {
        guard !styling, let storage = textStorage else { return }
        styling = true
        let selection = selectedRange()
        let clip = enclosingScrollView?.contentView
        let origin = clip?.bounds.origin
        let editingRange = window?.firstResponder === self ? selection : NSRange(location: NSNotFound, length: 0)
        MarkdownStyler.style(storage, activeRange: editingRange, source: sourceMode,
            width: textContainer?.size.width ?? 780, documentPath: documentPath, assetsRoot: model?.store?.assetsRoot, zoom: zoom, metaOffset: metaOffset)
        if let layout = layoutManager as? MarkdownLayoutManager {
            layout.deletedBlocks = []; layout.terminalDeletionHeight = 0
            layout.zoom = zoom; layout.metaOffset = metaOffset
            if showingDiff, let original = diffOriginal {
                let diff = InlineDiff(original: original, current: string)
                let top = DiffPresentation.apply(diff, storage: storage, layout: layout, width: textContainer?.size.width ?? 780, zoom: zoom)
                textContainerInset.height = 40 * zoom + top
            } else { textContainerInset.height = 40 * zoom }
        }
        typingAttributes = [.font: sourceMode ? NSFont.monospacedSystemFont(ofSize: 14 * zoom, weight: .regular) : MarkdownStyler.serif(MarkdownStyler.bodySize * zoom),
                            .foregroundColor: MarkdownStyler.textColor]
        if let origin { clip?.scroll(to: origin) }
        lastActiveParagraph = (string as NSString).paragraphRange(for: NSRange(location: min(selection.location, (string as NSString).length), length: 0))
        styling = false
        if let layout = layoutManager as? MarkdownLayoutManager, let container = textContainer {
            layout.ensureLayout(for: container)
            let height = max(enclosingScrollView?.contentView.bounds.height ?? 0,
                layout.usedRect(for: container).maxY + textContainerInset.height * 2 + layout.terminalDeletionHeight)
            super.setFrameSize(NSSize(width: frame.width, height: height))
        }
        updateMetaScroller()
        needsDisplay = true; enclosingScrollView?.verticalRulerView?.needsDisplay = true
    }
    private func updateMetaScroller() {
        if let scroll = enclosingScrollView, metaScroller.superview !== scroll { scroll.addSubview(metaScroller) }
        guard !sourceMode, let storage = textStorage, let layout = layoutManager, let container = textContainer else {
            metaScroller.isHidden = true; return
        }
        let ns = string as NSString
        var longest: CGFloat = 0; var lastRange: NSRange?
        storage.enumerateAttribute(.frontMatter, in: NSRange(location: 0, length: storage.length)) { value, range, _ in
            guard value != nil else { return }
            var position = range.location
            while position < NSMaxRange(range) {
                let line = ns.lineRange(for: NSRange(location: position, length: 0))
                longest = max(longest, (ns.substring(with: line).trimmingCharacters(in: .newlines) as NSString).size(withAttributes: [.font: NSFont.monospacedSystemFont(ofSize: 19 * 0.76 * self.zoom, weight: .regular)]).width + 10)
                lastRange = line; position = NSMaxRange(line)
            }
        }
        metaMaximumOffset = max(0, longest - container.size.width)
        let corrected = min(metaOffset, metaMaximumOffset)
        if abs(corrected - metaOffset) > 1 { metaOffset = corrected; restyle(); return }
        guard metaMaximumOffset > 0, let lastRange else { metaScroller.isHidden = true; return }
        let glyph = layout.glyphIndexForCharacter(at: lastRange.location)
        let rect = layout.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
        let documentRect = NSRect(x: textContainerOrigin.x, y: textContainerOrigin.y + rect.maxY + 2,
            width: container.size.width, height: 12)
        metaScroller.frame = enclosingScrollView?.convert(documentRect, from: self) ?? documentRect
        metaScroller.isEnabled = true
        metaScroller.knobProportion = container.size.width / longest
        metaScroller.doubleValue = Double(metaOffset / metaMaximumOffset)
        metaScroller.isHidden = enclosingScrollView.map { !$0.contentView.frame.contains(metaScroller.frame) } ?? true
    }
    @objc private func scrollMeta(_ sender: NSScroller) {
        var offset = CGFloat(sender.doubleValue) * metaMaximumOffset
        if sender.hitPart == .decrementPage { offset = metaOffset - (textContainer?.size.width ?? 200) * 0.8 }
        if sender.hitPart == .incrementPage { offset = metaOffset + (textContainer?.size.width ?? 200) * 0.8 }
        metaOffset = max(0, min(metaMaximumOffset, offset)); restyle()
    }
    override func scrollWheel(with event: NSEvent) {
        let delta = abs(event.scrollingDeltaX) > 0.1 ? event.scrollingDeltaX : event.modifierFlags.contains(.shift) ? event.scrollingDeltaY : 0
        if !sourceMode, delta != 0, isMetaPoint(convert(event.locationInWindow, from: nil)), metaMaximumOffset > 0 {
            metaOffset = max(0, min(metaMaximumOffset, metaOffset - delta)); restyle(); return
        }
        super.scrollWheel(with: event)
    }
    private func isMetaPoint(_ point: NSPoint) -> Bool {
        guard let storage = textStorage, let layout = layoutManager else { return false }
        var hit = false
        storage.enumerateAttribute(.frontMatter, in: NSRange(location: 0, length: storage.length)) { value, range, _ in
            guard value != nil else { return }
            let glyphs = layout.glyphRange(forCharacterRange: range, actualCharacterRange: nil)
            let rect = layout.boundingRect(forGlyphRange: glyphs, in: self.textContainer!)
            if point.y >= self.textContainerOrigin.y + rect.minY && point.y <= self.textContainerOrigin.y + rect.maxY + 12 { hit = true }
        }
        return hit
    }
    private func revealMetaCaret() {
        guard !sourceMode, window?.firstResponder === self, let storage = textStorage,
              selectedRange().location < storage.length,
              storage.attribute(.frontMatter, at: selectedRange().location, effectiveRange: nil) != nil else { return }
        let caret = selectedRange().location, ns = string as NSString
        let line = ns.lineRange(for: NSRange(location: caret, length: 0))
        let prefix = ns.substring(with: NSRange(location: line.location, length: caret - line.location))
        let x = (prefix as NSString).size(withAttributes: [.font: NSFont.monospacedSystemFont(ofSize: 19 * 0.76 * zoom, weight: .regular)]).width
        let width = (textContainer?.size.width ?? 200) - 20
        var offset = metaOffset
        if x < offset { offset = x }
        else if x > offset + width { offset = x - width }
        offset = max(0, min(metaMaximumOffset, offset))
        if abs(offset - metaOffset) > 1 { metaOffset = offset; restyle() }
    }
    func textView(_ textView: NSTextView, clickedOnLink link: Any, at charIndex: Int) -> Bool {
        let reference = (link as? String) ?? (link as? URL)?.absoluteString ?? ""
        if let url = FileService.linkedMarkdown(reference, from: documentPath) { model?.open(url); return true }
        if let url = URL(string: reference), ["https", "http", "mailto"].contains(url.scheme?.lowercased() ?? "") { NSWorkspace.shared.open(url); return true }
        return true
    }
    func textDidChange(_ notification: Notification) {
        guard !configuring && !styling else { return }
        rebuildLines(); flush()
        restyle(); revealMetaCaret()
    }
    func textViewDidChangeSelection(_ notification: Notification) {
        guard !configuring && !styling else { return }
        let range = (string as NSString).paragraphRange(for: NSRange(location: min(selectedRange().location, (string as NSString).length), length: 0))
        if range != lastActiveParagraph { restyle() }
        revealMetaCaret(); positionChanged()
    }
    func positionChanged() {
        guard !configuring && !styling else { return }
        enclosingScrollView?.verticalRulerView?.needsDisplay = true
        if !styling { updateMetaScroller() }
        positionWork?.cancel()
        let work = DispatchWorkItem { [weak self] in self?.flush() }
        positionWork = work; DispatchQueue.main.asyncAfter(deadline: .now() + 0.2, execute: work)
    }
    func flush() {
        guard !configuring, let id = draftID else { return }
        positionWork?.cancel()
        let caret = selectedRange().location
        let scroll = Double(enclosingScrollView?.contentView.bounds.origin.y ?? 0)
        guard string != storedContent || caret != storedSelection || abs(scroll - storedScroll) > 1 else { return }
        storedContent = string; storedSelection = caret; storedScroll = scroll
        model?.update(id: id, content: string, selection: caret, scroll: scroll)
    }
    private func rebuildLines() {
        lineStarts = [0]
        let units = Array(string.utf16)
        for index in units.indices where units[index] == 10 { lineStarts.append(index + 1) }
    }
    override func viewDidChangeEffectiveAppearance() {
        super.viewDidChangeEffectiveAppearance(); restyle()
    }
    override func becomeFirstResponder() -> Bool {
        let result = super.becomeFirstResponder()
        if result { DispatchQueue.main.async { [weak self] in self?.restyle(); self?.revealMetaCaret() } }
        return result
    }
    override func resignFirstResponder() -> Bool {
        let result = super.resignFirstResponder()
        if result { DispatchQueue.main.async { [weak self] in self?.restyle() } }
        return result
    }
    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        if showingDiff { (layoutManager as? MarkdownLayoutManager)?.drawDeletions(at: textContainerOrigin, dirtyRect: dirtyRect) }
        if showingDiff, (layoutManager as? MarkdownLayoutManager)?.deletedBlocks.isEmpty == false { return }
        guard string.isEmpty else { return }
        let origin = textContainerOrigin
        ("Comece a escrever…" as NSString).draw(at: origin, withAttributes: [
            .font: sourceMode ? NSFont.monospacedSystemFont(ofSize: 14, weight: .regular) : MarkdownStyler.serif(22),
            .foregroundColor: NSColor.tertiaryLabelColor
        ])
        ("Seu rascunho fica guardado neste Mac." as NSString).draw(
            at: NSPoint(x: origin.x, y: origin.y + 38),
            withAttributes: [.font: NSFont.systemFont(ofSize: 12), .foregroundColor: NSColor.tertiaryLabelColor])
    }
    override func copy(_ sender: Any?) {
        let text = (string as NSString).substring(with: selectedRange())
        NSPasteboard.general.clearContents(); NSPasteboard.general.setString(text, forType: .string)
    }
    override func paste(_ sender: Any?) {
        let pasteboard = NSPasteboard.general
        if let urls = pasteboard.readObjects(forClasses: [NSURL.self], options: nil) as? [URL],
           urls.contains(where: { NSImage(contentsOf: $0) != nil }) {
            for url in urls where NSImage(contentsOf: url) != nil { insertImage(url) }; return
        }
        if let data = pasteboard.data(forType: .png) { insertImageData(data, ext: "png"); return }
        if let data = pasteboard.data(forType: .tiff), let image = NSImage(data: data),
           let tiff = image.tiffRepresentation, let bitmap = NSBitmapImageRep(data: tiff),
           let png = bitmap.representation(using: .png, properties: [:]) { insertImageData(png, ext: "png"); return }
        if let plain = pasteboard.string(forType: .string) { insertText(plain, replacementRange: selectedRange()) }
    }
    func insertImage(_ url: URL) {
        do {
            guard NSImage(contentsOf: url) != nil else { throw TextMDError.invalidAsset }
            let data = try Data(contentsOf: url)
            insertImageData(data, ext: url.pathExtension.lowercased())
        } catch { model?.report(error) }
    }
    private func insertImageData(_ data: Data, ext: String) {
        do {
            guard let store = model?.store, NSImage(data: data) != nil else { throw TextMDError.invalidAsset }
            let ref = try FileService.storeImage(data, extension: ext, assetsRoot: store.assetsRoot)
            insertText("\n\n![Imagem](\(ref))\n\n", replacementRange: selectedRange())
        } catch { model?.report(error) }
    }
    override func performDragOperation(_ sender: NSDraggingInfo) -> Bool {
        if let urls = sender.draggingPasteboard.readObjects(forClasses: [NSURL.self], options: nil) as? [URL],
           urls.contains(where: { NSImage(contentsOf: $0) != nil }) {
            for url in urls where NSImage(contentsOf: url) != nil { insertImage(url) }; return true
        }
        return super.performDragOperation(sender)
    }
    override func insertNewline(_ sender: Any?) {
        let ns = string as NSString
        let selection = selectedRange()
        let line = ns.paragraphRange(for: NSRange(location: min(selection.location, max(0, ns.length - 1)), length: 0))
        let before = ns.substring(with: NSRange(location: line.location, length: max(0, selection.location - line.location)))
        let regex = try? NSRegularExpression(pattern: "^([ \\t]*)([-*+] |[0-9]+[.)] )(\\[[ xX]\\] )?")
        let match = regex?.firstMatch(in: before, range: NSRange(location: 0, length: (before as NSString).length))
        if let match, match.range.length == (before as NSString).length {
            insertText("", replacementRange: NSRange(location: line.location, length: before.utf16.count)); return
        }
        super.insertNewline(sender)
        if let match {
            let nsBefore = before as NSString
            var prefix = nsBefore.substring(with: match.range(at: 1)) + nsBefore.substring(with: match.range(at: 2))
            if match.range(at: 3).location != NSNotFound { prefix += "[ ] " }
            if let number = Int(nsBefore.substring(with: match.range(at: 2)).trimmingCharacters(in: CharacterSet(charactersIn: ".) "))) {
                prefix = nsBefore.substring(with: match.range(at: 1)) + "\(number + 1). "
            }
            insertText(prefix, replacementRange: selectedRange())
        }
    }
    override func mouseDown(with event: NSEvent) {
        if !sourceMode, let storage = textStorage, storage.length > 0, let layout = layoutManager, layout.numberOfGlyphs > 0, let container = textContainer {
            let point = convert(event.locationInWindow, from: nil)
            let local = NSPoint(x: point.x - textContainerOrigin.x, y: point.y - textContainerOrigin.y)
            let glyph = layout.glyphIndex(for: local, in: container)
            let character = min(layout.characterIndexForGlyph(at: glyph), max(0, storage.length - 1))
            if storage.length > 0, let link = storage.attribute(.link, at: character, effectiveRange: nil),
               let url = FileService.linkedMarkdown((link as? String) ?? "", from: documentPath),
               layout.boundingRect(forGlyphRange: NSRange(location: glyph, length: 1), in: container).insetBy(dx: -2, dy: -3).contains(local) {
                model?.open(url); return
            }
            if storage.length > 0, storage.attribute(.taskBox, at: character, effectiveRange: nil) != nil,
               local.x >= 0, local.x <= 22 * zoom {
                let line = (string as NSString).lineRange(for: NSRange(location: character, length: 0))
                let raw = (string as NSString).substring(with: line)
                if let match = (try? NSRegularExpression(pattern: "\\[([ xX])\\]"))?.firstMatch(in: raw, range: NSRange(location: 0, length: (raw as NSString).length)) {
                    let pos = line.location + match.range(at: 1).location
                    let checked = (string as NSString).substring(with: NSRange(location: pos, length: 1)) != " "
                    insertText(checked ? " " : "x", replacementRange: NSRange(location: pos, length: 1)); return
                }
            }
        }
        super.mouseDown(with: event)
    }
    func showFind() {
        let item = NSMenuItem(); item.tag = NSTextFinder.Action.showFindInterface.rawValue
        performTextFinderAction(item)
    }
    func format(_ action: MarkdownAction) {
        window?.makeFirstResponder(self)
        let selection = selectedRange()
        let selected = (string as NSString).substring(with: selection)
        var replacement = ""; var cursorOffset: Int?
        switch action {
        case .bold: replacement = "**" + selected + "**"; if selected.isEmpty { cursorOffset = 2 }
        case .italic: replacement = "*" + selected + "*"; if selected.isEmpty { cursorOffset = 1 }
        case .link: replacement = "[" + (selected.isEmpty ? "texto" : selected) + "](https://)"
        case .code: replacement = selected.contains("\n") ? "\n```\n" + selected + "\n```\n" : "`" + selected + "`"; if selected.isEmpty { cursorOffset = 1 }
        case .heading, .bullet, .task:
            let prefix = action == .heading ? "## " : action == .bullet ? "- " : "- [ ] "
            let paragraph = (string as NSString).paragraphRange(for: selection)
            let lines = (string as NSString).substring(with: paragraph).components(separatedBy: "\n")
            replacement = lines.enumerated().map { index, line in
                if line.isEmpty && index == lines.count - 1 && lines.count > 1 { return "" }
                return prefix + line
            }.joined(separator: "\n")
            insertText(replacement, replacementRange: paragraph); return
        case .table:
            replacement = "\n\n| Coluna 1 | Coluna 2 |\n| -------- | -------- |\n| Texto    | Texto    |\n\n"
        }
        insertText(replacement, replacementRange: selection)
        if let cursorOffset { setSelectedRange(NSRange(location: selection.location + cursorOffset, length: 0)) }
    }
}

@MainActor
final class LineNumberRuler: NSRulerView {
    weak var editor: MarkdownTextView?
    init(textView: MarkdownTextView) {
        editor = textView
        super.init(scrollView: textView.enclosingScrollView, orientation: .verticalRuler)
        clientView = textView; ruleThickness = 48
    }
    required init(coder: NSCoder) { super.init(coder: coder) }
    override func drawHashMarksAndLabels(in rect: NSRect) {
        // AppKit supplies a dirty rect that can extend into the document area.
        // Never paint it: doing so covers the editor when the ruler is enabled.
        guard let editor, let layout = editor.layoutManager,
              let clip = editor.enclosingScrollView?.contentView else { return }
        let attrs: [NSAttributedString.Key: Any] = [.font: NSFont.monospacedDigitSystemFont(ofSize: 11, weight: .regular), .foregroundColor: NSColor.tertiaryLabelColor]
        for (index, location) in editor.lineStarts.enumerated() {
            let y: CGFloat
            if location < (editor.string as NSString).length {
                let glyph = layout.glyphIndexForCharacter(at: location)
                y = layout.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil).minY
            } else { y = layout.extraLineFragmentRect.minY }
            let visibleY = y + editor.textContainerOrigin.y - clip.bounds.minY
            guard visibleY >= -20 && visibleY <= bounds.height + 20 else { continue }
            let label = "\(index + 1)" as NSString
            label.draw(at: NSPoint(x: ruleThickness - label.size(withAttributes: attrs).width - 10, y: visibleY + 2), withAttributes: attrs)
        }
    }
}
