import Foundation

public struct FileSearchResult: Identifiable, Sendable {
    public let path: String
    public let excerpt: String
    public var id: String { path }
}
public enum FileSearch {
    public static func scan(query: String, paths: [String], checkCancellation: () throws -> Void = { try Task.checkCancellation() }) throws -> (results: [FileSearchResult], errors: [String]) {
        var found: [FileSearchResult] = [], errors: [String] = [], visited = Set<String>()
        for path in paths {
            try checkCancellation()
            guard let iterator = FileManager.default.enumerator(at: URL(fileURLWithPath: path),
                includingPropertiesForKeys: [.isDirectoryKey, .isSymbolicLinkKey], options: [], errorHandler: { url, error in
                    errors.append(url.lastPathComponent + ": " + error.localizedDescription); return true
                }) else { errors.append(path); continue }
            while let url = iterator.nextObject() as? URL {
                try checkCancellation()
                if [".git", ".build", "node_modules"].contains(url.lastPathComponent) { iterator.skipDescendants(); continue }
                if (try? url.resourceValues(forKeys: [.isSymbolicLinkKey]).isSymbolicLink) == true { iterator.skipDescendants(); continue }
                guard ["md", "markdown"].contains(url.pathExtension.lowercased()), visited.insert(url.path).inserted else { continue }
                do {
                    let text = try String(contentsOf: url, encoding: .utf8)
                    try checkCancellation()
                    if url.lastPathComponent.localizedStandardContains(query) || text.localizedStandardContains(query) {
                        let line = text.components(separatedBy: .newlines).first { $0.localizedStandardContains(query) } ?? ""
                        found.append(FileSearchResult(path: url.path, excerpt: String(line.prefix(160))))
                    }
                } catch is CancellationError { throw CancellationError() }
                catch { errors.append(url.lastPathComponent + ": " + error.localizedDescription) }
            }
        }
        return (found.sorted { $0.path.localizedStandardCompare($1.path) == .orderedAscending }, errors)
    }
}
