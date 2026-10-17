import AppKit
import Foundation

let destination = CommandLine.arguments[1]
let size = NSSize(width: 1024, height: 1024)
let image = NSImage(size: size)
image.lockFocus()
let background = NSBezierPath(roundedRect: NSRect(x: 64, y: 64, width: 896, height: 896), xRadius: 190, yRadius: 190)
NSColor(calibratedWhite: 0.105, alpha: 1).setFill(); background.fill()
NSColor(calibratedWhite: 0.27, alpha: 1).setStroke(); background.lineWidth = 4; background.stroke()
let font = NSFont(name: "Georgia-Italic", size: 370) ?? NSFont.systemFont(ofSize: 370, weight: .medium)
let title = NSAttributedString(string: "md", attributes: [.font: font, .foregroundColor: NSColor.white])
let measured = title.size()
title.draw(at: NSPoint(x: (size.width - measured.width) / 2 - 10, y: (size.height - measured.height) / 2 + 25))
let rule = NSBezierPath(); rule.move(to: NSPoint(x: 290, y: 287)); rule.line(to: NSPoint(x: 734, y: 287))
rule.lineWidth = 7; NSColor(calibratedWhite: 0.72, alpha: 1).setStroke(); rule.stroke()
image.unlockFocus()
if let tiff = image.tiffRepresentation, let bitmap = NSBitmapImageRep(data: tiff),
   let png = bitmap.representation(using: .png, properties: [:]) {
    try png.write(to: URL(fileURLWithPath: destination))
}
