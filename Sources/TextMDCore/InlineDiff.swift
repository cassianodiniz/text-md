import Foundation

public struct DiffDeletion: Equatable, Sendable {
    /// UTF-16 offset in the CURRENT text, before which old text is drawn.
    public let anchor: Int
    public let text: String
}
public struct InlineDiff: Equatable, Sendable {
    public let additions: [NSRange]
    public let deletions: [DiffDeletion]
    public init(original: String, current: String) {
        let old = Self.lines(original), new = Self.lines(current)
        let changes = new.difference(from: old)
        var removed = Set<Int>(), inserted = Set<Int>()
        for change in changes {
            switch change {
            case .remove(let offset, _, _): removed.insert(offset)
            case .insert(let offset, _, _): inserted.insert(offset)
            }
        }
        var additions: [NSRange] = [], deletions: [DiffDeletion] = []
        var a = 0, b = 0, offset = 0, pending = ""
        func flush() {
            if !pending.isEmpty { deletions.append(DiffDeletion(anchor: offset, text: pending)); pending = "" }
        }
        while a < old.count || b < new.count {
            if a < old.count && removed.contains(a) { pending += old[a]; a += 1 }
            else if b < new.count && inserted.contains(b) {
                flush()
                let length = new[b].utf16.count
                additions.append(NSRange(location: offset, length: length))
                offset += length; b += 1
            } else {
                flush()
                if b < new.count { offset += new[b].utf16.count; b += 1 }
                if a < old.count { a += 1 }
            }
        }
        flush()
        self.additions = additions; self.deletions = deletions
    }
    private static func lines(_ value: String) -> [String] {
        let ns = value as NSString
        var result: [String] = [], position = 0
        while position < ns.length {
            let range = ns.lineRange(for: NSRange(location: position, length: 0))
            result.append(ns.substring(with: range)); position = NSMaxRange(range)
        }
        return result
    }
}
