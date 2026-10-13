import Foundation
import CryptoKit

public enum FileService {
    public static func validName(_ value: String) throws -> String {
        let name = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty, !name.contains("/"), !name.contains(":"), name != ".", name != "..",
              name.rangeOfCharacter(from: .controlCharacters) == nil else {
            throw TextMDError.invalidDestination("Escolha um nome válido.")
        }
        return name
    }
    public static func renamedDestination(for source: URL, name: String) throws -> URL {
        var name = try validName(name)
        if (name as NSString).pathExtension.isEmpty && !source.pathExtension.isEmpty { name += "." + source.pathExtension }
        return source.deletingLastPathComponent().appendingPathComponent(name)
    }
    public static func suggestedFilename(for draft: Draft) throws -> String {
        let cleaned = draft.title.components(separatedBy: CharacterSet(charactersIn: "/:").union(.controlCharacters)).joined(separator: "-")
        let name = try validName(cleaned)
        return (name as NSString).pathExtension.isEmpty ? name + ".md" : name
    }

    public static func isEditable(_ url: URL) -> Bool {
        ["md", "markdown", "txt"].contains(url.pathExtension.lowercased()) ||
        ["LICENSE", "README", "NOTICE", "CHANGELOG", ".GITIGNORE"].contains(url.lastPathComponent.uppercased())
    }
    public static func linkedMarkdown(_ reference: String, from documentPath: String?) -> URL? {
        guard let components = URLComponents(string: reference.trimmingCharacters(in: CharacterSet(charactersIn: "<>"))),
              components.scheme == nil || components.scheme == "file", components.host == nil || components.host?.isEmpty == true || components.host == "localhost" else { return nil }
        let path = components.path
        guard !path.isEmpty, ["md", "markdown"].contains((path as NSString).pathExtension.lowercased()) else { return nil }
        if path.hasPrefix("/") { return URL(fileURLWithPath: path).standardizedFileURL }
        guard let documentPath else { return nil }
        return URL(fileURLWithPath: documentPath).deletingLastPathComponent().appendingPathComponent(path).standardizedFileURL
    }
    /// Compare identities without rewriting the user-selected path or image base.
    /// macOS can expose the same file under /var and /private/var.
    public static func documentKey(_ url: URL) -> String {
        url.standardizedFileURL.resolvingSymlinksInPath().standardizedFileURL.path
    }
    public static func contentData(_ text: String, utf8BOM: Bool) -> Data {
        (utf8BOM ? Data([0xef, 0xbb, 0xbf]) : Data()) + Data(text.utf8)
    }
    public static func hash(_ data: Data) -> String {
        SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    }
    public static func open(_ url: URL, now: Date = Date()) throws -> Draft {
        let data = try Data(contentsOf: url)
        guard let text = String(data: data, encoding: .utf8) else { throw TextMDError.invalidUTF8 }
        var draft = Draft(content: text, sourcePath: url.standardizedFileURL.path,
                          baselineHash: hash(data), now: now)
        draft.hasUTF8BOM = data.starts(with: [0xef, 0xbb, 0xbf])
        draft.bookmark = try? url.bookmarkData(options: [], includingResourceValuesForKeys: nil, relativeTo: nil)
        return draft
    }
    public static func diskState(for draft: Draft) -> DiskState {
        guard let path = draft.sourcePath else { return .unchanged }
        do {
            let data = try Data(contentsOf: URL(fileURLWithPath: path))
            if hash(data) == draft.baselineHash { return .unchanged }
            guard let text = String(data: data, encoding: .utf8) else { return .inaccessible(TextMDError.invalidUTF8.localizedDescription) }
            return .changed(text)
        } catch let error as NSError {
            if error.domain == NSCocoaErrorDomain && (error.code == NSFileReadNoSuchFileError || error.code == NSFileNoSuchFileError) {
                return .missing
            }
            return .inaccessible(error.localizedDescription)
        }
    }
    /// Reload file content while preserving the draft identity and sidebar state.
    public static func reload(_ draft: Draft) throws -> Draft {
        guard let path = draft.sourcePath else { throw TextMDError.missing }
        var current = try open(URL(fileURLWithPath: path))
        current.id = draft.id; current.createdAt = draft.createdAt; current.pinned = draft.pinned
        current.displayName = draft.displayName; current.hiddenFromSidebar = draft.hiddenFromSidebar
        current.hasWrittenContent = draft.hasWrittenContent; current.lastSnapshotAt = draft.lastSnapshotAt
        current.selection = min(draft.selection, (current.content as NSString).length)
        current.scrollPosition = draft.scrollPosition
        return current
    }
    /// Content is checked again inside the coordinated write, including when the
    /// original changed after the UI's last check. Symlinks resolve to the real file.
    public static func save(_ draft: Draft, to destination: URL? = nil,
                            overwriteCopy: Bool = false, assetsRoot: URL) throws -> Draft {
        guard let target = destination ?? draft.sourcePath.map({ URL(fileURLWithPath: $0) }) else {
            throw TextMDError.invalidDestination("Escolha uma pasta e um nome.")
        }
        let original = draft.sourcePath.map { URL(fileURLWithPath: $0).standardizedFileURL }
        let writesOriginal = original?.resolvingSymlinksInPath() == target.standardizedFileURL.resolvingSymlinksInPath()
        let actualTarget = target.resolvingSymlinksInPath()
        var result = draft
        var failure: Error?
        var coordinationError: NSError?
        let coordinator = NSFileCoordinator()
        coordinator.coordinate(writingItemAt: actualTarget, options: .forReplacing, error: &coordinationError) { url in
            do {
                if writesOriginal {
                    guard FileManager.default.fileExists(atPath: url.path) else { throw TextMDError.missing }
                    guard hash(try Data(contentsOf: url)) == draft.baselineHash else { throw TextMDError.conflict }
                } else if FileManager.default.fileExists(atPath: url.path), !overwriteCopy {
                    throw TextMDError.invalidDestination("Já existe um arquivo com esse nome. Escolha outro nome.")
                }
                if let original, original.deletingLastPathComponent().resolvingSymlinksInPath() != url.deletingLastPathComponent().resolvingSymlinksInPath() {
                    try preserveRelativeImages(in: [draft.content], from: original, to: url)
                }
                let prepared = try exportAssets(in: draft.content, to: url, assetsRoot: assetsRoot)
                let output = contentData(prepared, utf8BOM: draft.hasUTF8BOM == true)
                try output.write(to: url, options: .atomic)
                result.content = prepared
                result.sourcePath = target.standardizedFileURL.path
                result.baselineHash = hash(output)
                result.modifiedAt = Date()
                result.bookmark = try? target.bookmarkData(options: [], includingResourceValuesForKeys: nil, relativeTo: nil)
            } catch { failure = error }
        }
        if let coordinationError { throw coordinationError }
        if let failure { throw failure }
        return result
    }

    /// Moving the document is explicit, but must not rewrite its Markdown. Copy
    /// local relative image dependencies to the same relative locations first.
    public static func moveDocument(from source: URL, to target: URL, draftContents: [String] = []) throws {
        let fm = FileManager.default
        if fm.fileExists(atPath: target.path) {
            let sourceInfo = try fm.attributesOfItem(atPath: source.path)
            let targetInfo = try fm.attributesOfItem(atPath: target.path)
            let onlyCaseChanged = source.deletingLastPathComponent() == target.deletingLastPathComponent() &&
                source.lastPathComponent != target.lastPathComponent &&
                source.lastPathComponent.caseInsensitiveCompare(target.lastPathComponent) == .orderedSame
            if onlyCaseChanged, sourceInfo[.systemFileNumber] as? NSNumber == targetInfo[.systemFileNumber] as? NSNumber,
               sourceInfo[.systemNumber] as? NSNumber == targetInfo[.systemNumber] as? NSNumber {
                try fm.moveItem(at: source, to: target); return
            }
            throw TextMDError.invalidDestination("Já existe um arquivo com esse nome. Nenhum arquivo foi substituído.")
        }
        let original = try open(source)
        try preserveRelativeImages(in: [original.content] + draftContents, from: source, to: target)
        try fm.moveItem(at: source, to: target)
    }
    private static func preserveRelativeImages(in contents: [String], from source: URL, to target: URL) throws {
        let oldFolder = source.deletingLastPathComponent().resolvingSymlinksInPath()
        let newFolder = target.deletingLastPathComponent().resolvingSymlinksInPath()
        guard oldFolder != newFolder else { return }
        let regex = try NSRegularExpression(pattern: "!\\[[^\\]]*\\]\\(<?([^)>]+)>?\\)")
        var copies: [URL: URL] = [:]
        for content in contents {
            let ns = content as NSString
            for match in regex.matches(in: content, range: NSRange(location: 0, length: ns.length)) {
                let reference = ns.substring(with: match.range(at: 1)).trimmingCharacters(in: .whitespaces)
                guard !reference.contains(":"), !reference.hasPrefix("/") else { continue }
                let decoded = reference.removingPercentEncoding ?? reference
                let image = oldFolder.appendingPathComponent(decoded).resolvingSymlinksInPath()
                let destination = newFolder.appendingPathComponent(decoded).resolvingSymlinksInPath()
                guard image != destination, FileManager.default.fileExists(atPath: image.path) else { continue }
                guard ["png", "jpg", "jpeg", "gif", "tif", "tiff", "webp", "heic", "svg", "pdf"].contains(image.pathExtension.lowercased()) else { continue }
                guard destination.path.hasPrefix(newFolder.path + "/") else {
                    throw TextMDError.invalidDestination("Uma imagem usa um caminho fora desta pasta. Para preservar suas referências, mantenha o documento na mesma pasta ou mova a pasta inteira pelo Finder.")
                }
                if FileManager.default.fileExists(atPath: destination.path) {
                    guard try Data(contentsOf: image) == Data(contentsOf: destination) else {
                        throw TextMDError.invalidDestination("Há uma imagem diferente com o mesmo nome na pasta de destino. O documento não foi movido nem substituído.")
                    }
                } else { copies[destination] = image }
            }
        }
        for (destination, image) in copies {
            try FileManager.default.createDirectory(at: destination.deletingLastPathComponent(), withIntermediateDirectories: true)
            try FileManager.default.copyItem(at: image, to: destination)
        }
    }

    public static func storeImage(_ data: Data, extension ext: String, assetsRoot: URL) throws -> String {
        let allowed = ["png", "jpg", "jpeg", "gif", "tiff", "webp", "heic"]
        guard allowed.contains(ext.lowercased()), !data.isEmpty else { throw TextMDError.invalidAsset }
        try FileManager.default.createDirectory(at: assetsRoot, withIntermediateDirectories: true)
        let name = UUID().uuidString.lowercased() + "." + ext.lowercased()
        try data.write(to: assetsRoot.appendingPathComponent(name), options: .atomic)
        return "textmd-asset:" + name
    }
    public static func assetURL(_ reference: String, assetsRoot: URL) throws -> URL {
        let name = String(reference.dropFirst("textmd-asset:".count))
        guard reference.hasPrefix("textmd-asset:"), !name.isEmpty,
              name == URL(fileURLWithPath: name).lastPathComponent,
              !name.contains("/"), !name.contains("\\"), !name.contains("..") else { throw TextMDError.invalidAsset }
        return assetsRoot.appendingPathComponent(name)
    }
    private static func exportAssets(in content: String, to target: URL, assetsRoot: URL) throws -> String {
        let regex = try NSRegularExpression(pattern: "textmd-asset:([A-Za-z0-9._-]+)")
        let ns = content as NSString
        let matches = regex.matches(in: content, range: NSRange(location: 0, length: ns.length))
        var result = content
        let folderName = target.deletingPathExtension().lastPathComponent + ".images"
        let folder = target.deletingLastPathComponent().appendingPathComponent(folderName, isDirectory: true)
        for match in matches.reversed() {
            let ref = ns.substring(with: match.range)
            let source = try assetURL(ref, assetsRoot: assetsRoot)
            guard FileManager.default.fileExists(atPath: source.path) else { throw TextMDError.invalidAsset }
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            let destination = folder.appendingPathComponent(source.lastPathComponent)
            if !FileManager.default.fileExists(atPath: destination.path) {
                try FileManager.default.copyItem(at: source, to: destination)
            } else if try Data(contentsOf: source) != Data(contentsOf: destination) {
                throw TextMDError.invalidDestination("Uma imagem com o mesmo nome já existe e tem conteúdo diferente.")
            }
            let relative = folderName + "/" + source.lastPathComponent
            let safe = relative.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? relative
            if let range = Range(match.range, in: result) { result.replaceSubrange(range, with: safe) }
        }
        return result
    }
}
