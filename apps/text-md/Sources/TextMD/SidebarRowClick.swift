import AppKit
import SwiftUI

/// Distinguish a normal click from rename before opening changes the sidebar or
/// activates another window. Capture Command from the mouse event itself.
struct SidebarRowClick: NSViewRepresentable {
    let open: (Bool) -> Void
    let rename: () -> Void
    func makeNSView(context: Context) -> ClickView {
        let view = ClickView(); view.setAccessibilityElement(false); return view
    }
    func updateNSView(_ view: ClickView, context: Context) { view.open = open; view.rename = rename }
    final class ClickView: NSView {
        var open: ((Bool) -> Void)?
        var rename: (() -> Void)?
        private var pending: DispatchWorkItem?
        private var start: NSPoint?
        override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
        override func mouseDown(with event: NSEvent) { pending?.cancel(); start = event.locationInWindow }
        override func mouseDragged(with event: NSEvent) {
            if let start, hypot(event.locationInWindow.x - start.x, event.locationInWindow.y - start.y) > 5 { self.start = nil }
        }
        override func mouseUp(with event: NSEvent) {
            guard start != nil else { return }; start = nil
            if event.clickCount >= 2 { pending?.cancel(); rename?(); return }
            let newTab = event.modifierFlags.contains(.command)
            let work = DispatchWorkItem { [weak self] in self?.open?(newTab) }
            pending = work
            DispatchQueue.main.asyncAfter(deadline: .now() + NSEvent.doubleClickInterval, execute: work)
        }
        override func viewDidMoveToWindow() { super.viewDidMoveToWindow(); if window == nil { pending?.cancel() } }
        deinit { pending?.cancel() }
    }
}
