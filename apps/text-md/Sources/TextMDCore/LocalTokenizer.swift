import Foundation

public enum TokenizerError: LocalizedError {
    case unavailable, invalidData, unmatchedText
    public var errorDescription: String? { "Não foi possível contar tokens com os dados locais. Tente outro perfil." }
}

/// Exact ordinary-text BPE using OpenAI's bundled ranks. No network, runtime
/// Python, API key or character-based approximation is used by the application.
public final class LocalTokenizer: @unchecked Sendable {
    private let ranks: [Data: Int]
    private let regex: NSRegularExpression
    public init(profile: TokenProfile) throws {
        let data = try Data(contentsOf: Self.resource(profile.rawValue, extension: "tiktoken"))
        let expected = profile == .o200k ? "446a9538cb6c348e3516120d7c08b09f57c36495e2acfffe59a5bf8b0cfb1a2d" : "223921b76ee99bde995b7ff738513eef100fb51d18c93597a113bcffe865b2a7"
        guard FileService.hash(data) == expected, let text = String(data: data, encoding: .utf8) else { throw TokenizerError.invalidData }
        var ranks: [Data: Int] = [:]
        for line in text.split(separator: "\n") {
            let pair = line.split(separator: " ")
            guard pair.count == 2, let bytes = Data(base64Encoded: String(pair[0])), let rank = Int(pair[1]) else { throw TokenizerError.invalidData }
            ranks[bytes] = rank
        }
        guard ranks.count == (profile == .o200k ? 199998 : 100256) else { throw TokenizerError.invalidData }
        self.ranks = ranks
        let patterns = try JSONDecoder().decode([String: String].self, from: Data(contentsOf: Self.resource("patterns", extension: "json")))
        guard let pattern = patterns[profile.rawValue] else { throw TokenizerError.invalidData }
        regex = try NSRegularExpression(pattern: pattern)
    }
    private static func resource(_ name: String, extension ext: String) throws -> URL {
        // SwiftPM's development fallback isn't sufficient for a relocated .app.
        if let root = Bundle.main.resourceURL,
           let bundle = Bundle(url: root.appendingPathComponent("text-md_TextMDCore.bundle")),
           let url = bundle.url(forResource: name, withExtension: ext) { return url }
        if let url = Bundle.module.url(forResource: name, withExtension: ext) { return url }
        throw TokenizerError.unavailable
    }
    public func count(_ text: String) throws -> Int {
        let ns = text as NSString
        var count = 0, covered = 0
        for match in regex.matches(in: text, range: NSRange(location: 0, length: ns.length)) {
            if Task.isCancelled { throw CancellationError() }
            guard match.range.location == covered else { throw TokenizerError.unmatchedText }
            covered = NSMaxRange(match.range)
            count += countPiece(Data(ns.substring(with: match.range).utf8))
        }
        guard covered == ns.length else { throw TokenizerError.unmatchedText }
        return count
    }
    private struct Candidate {
        let rank: Int, left: Int, right: Int, leftVersion: Int, rightVersion: Int
    }
    private func countPiece(_ data: Data) -> Int {
        if ranks[data] != nil { return 1 }
        let bytes = Array(data), n = data.count
        guard n > 1 else { return n }
        var next = Array(1...n), previous = Array(-1..<(n - 1)), end = Array(1...n), versions = Array(repeating: 0, count: n)
        var heap: [Candidate] = [], remaining = n
        func less(_ a: Candidate, _ b: Candidate) -> Bool { a.rank == b.rank ? a.left < b.left : a.rank < b.rank }
        func push(_ left: Int) {
            guard left >= 0, left < n, next[left] < n else { return }
            let right = next[left]
            guard let rank = ranks[Data(bytes[left..<end[right]])] else { return }
            heap.append(Candidate(rank: rank, left: left, right: right, leftVersion: versions[left], rightVersion: versions[right]))
            var i = heap.count - 1
            while i > 0 {
                let p = (i - 1) / 2
                guard less(heap[i], heap[p]) else { break }
                heap.swapAt(i, p); i = p
            }
        }
        func pop() -> Candidate? {
            guard !heap.isEmpty else { return nil }
            if heap.count == 1 { return heap.removeLast() }
            let first = heap[0]; heap[0] = heap.removeLast()
            var i = 0
            while i * 2 + 1 < heap.count {
                let l = i * 2 + 1, r = l + 1
                let child = r < heap.count && less(heap[r], heap[l]) ? r : l
                guard less(heap[child], heap[i]) else { break }
                heap.swapAt(i, child); i = child
            }
            return first
        }
        for i in 0..<(n - 1) { push(i) }
        while let pair = pop() {
            let l = pair.left, r = pair.right
            guard versions[l] == pair.leftVersion, versions[r] == pair.rightVersion, next[l] == r else { continue }
            end[l] = end[r]; next[l] = next[r]
            if next[r] < n { previous[next[r]] = l }
            versions[l] += 1; versions[r] += 1; remaining -= 1
            push(previous[l]); push(l)
        }
        return remaining
    }
}

public actor TokenizerRegistry {
    public static let shared = TokenizerRegistry()
    private var loaded: [TokenProfile: LocalTokenizer] = [:]
    public func tokenizer(_ profile: TokenProfile) throws -> LocalTokenizer {
        if let tokenizer = loaded[profile] { return tokenizer }
        let tokenizer = try LocalTokenizer(profile: profile); loaded[profile] = tokenizer
        return tokenizer
    }
}
