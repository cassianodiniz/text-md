import AppKit
import SwiftUI

extension NSPasteboard.PasteboardType { static let textmdTab = Self("com.cassiano.text-md.tab") }

/// AppKit supplies native drag images, cross-window moves and drag-out handling.
struct TabDragHandle: NSViewRepresentable {
    let model: AppModel
    let id: UUID
    let title: String
    func makeNSView(context: Context) -> DragView { let view = DragView(); view.setAccessibilityElement(false); return view }
    func updateNSView(_ view: DragView, context: Context) {
        view.model = model; view.id = id; view.title = title
        view.registerForDraggedTypes([.textmdTab])
    }
    final class DragView: NSView, NSDraggingSource {
        weak var model: AppModel?
        var id: UUID?
        var title = ""
        private var down: NSEvent?
        private var dragging = false
        override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
        override func mouseDown(with event: NSEvent) { down = event; dragging = false }
        override func mouseUp(with event: NSEvent) { if !dragging, let id { model?.select(id) }; down = nil }
        override func mouseDragged(with event: NSEvent) {
            guard !dragging, let down, let id else { return }
            let delta = hypot(event.locationInWindow.x - down.locationInWindow.x, event.locationInWindow.y - down.locationInWindow.y)
            guard delta > 5 else { return }
            dragging = true
            let pasteboard = NSPasteboardItem(); pasteboard.setString(id.uuidString, forType: .textmdTab)
            let item = NSDraggingItem(pasteboardWriter: pasteboard)
            let image = NSImage(size: bounds.size, flipped: false) { rect in
                NSColor.windowBackgroundColor.setFill(); NSBezierPath(roundedRect: rect, xRadius: 7, yRadius: 7).fill()
                (self.title as NSString).draw(in: rect.insetBy(dx: 18, dy: 6), withAttributes: [.font: NSFont.systemFont(ofSize: 12), .foregroundColor: NSColor.labelColor])
                return true
            }
            item.setDraggingFrame(bounds, contents: image)
            beginDraggingSession(with: [item], event: event, source: self)
        }
        func draggingSession(_ session: NSDraggingSession, sourceOperationMaskFor context: NSDraggingContext) -> NSDragOperation { .move }
        func draggingSession(_ session: NSDraggingSession, endedAt screenPoint: NSPoint, operation: NSDragOperation) {
            defer { dragging = false; down = nil }
            guard operation.isEmpty, NSEvent.pressedMouseButtons == 0, let id, let coordinator = model?.coordinator else { return }
            // Escape cancels while the button is still held. A failed drop inside
            // an existing app window also leaves the source untouched.
            if !coordinator.models.contains(where: { $0.nativeWindow?.frame.contains(screenPoint) == true }) { coordinator.detachTab(id) }
        }
        private func incoming(_ sender: NSDraggingInfo) -> UUID? {
            sender.draggingPasteboard.string(forType: .textmdTab).flatMap(UUID.init(uuidString:))
        }
        override func draggingEntered(_ sender: NSDraggingInfo) -> NSDragOperation {
            guard let id = incoming(sender), model?.coordinator?.owner(of: id) != nil else { return [] }
            return .move
        }
        override func prepareForDragOperation(_ sender: NSDraggingInfo) -> Bool { incoming(sender) != nil }
        override func performDragOperation(_ sender: NSDraggingInfo) -> Bool {
            guard let moving = incoming(sender), let model, model.coordinator?.owner(of: moving) != nil else { return false }
            model.coordinator?.moveTab(moving, to: model, before: id); return true
        }
    }
}
