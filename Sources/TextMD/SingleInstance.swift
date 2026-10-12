import AppKit
import Darwin

/// A kernel-held lock prevents two writers even under simultaneous/direct launches.
@MainActor
enum SingleInstance {
    private static var descriptor: Int32 = -1
    static func prepare() {
        do {
            let root = try AppLibrary.rootURL()
            try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
            let path = root.appendingPathComponent("instance.lock").path
            descriptor = Darwin.open(path, O_RDWR | O_CREAT | O_CLOEXEC, S_IRUSR | S_IWUSR)
            guard descriptor >= 0 else { throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno)) }
            if flock(descriptor, LOCK_EX | LOCK_NB) != 0 {
                let owner = (try? String(contentsOfFile: path, encoding: .utf8)).flatMap { Int32($0.trimmingCharacters(in: .whitespacesAndNewlines)) }
                if let owner, let app = NSRunningApplication(processIdentifier: owner) {
                    app.activate(options: [])
                    let urls = CommandLine.arguments.dropFirst().filter { !$0.hasPrefix("-") }.map { URL(fileURLWithPath: $0) }.filter { FileManager.default.fileExists(atPath: $0.path) }
                    if !urls.isEmpty {
                        let target = NSAppleEventDescriptor(processIdentifier: owner)
                        let event = NSAppleEventDescriptor(eventClass: AEEventClass(kCoreEventClass), eventID: AEEventID(kAEOpenDocuments),
                            targetDescriptor: target, returnID: AEReturnID(kAutoGenerateReturnID), transactionID: AETransactionID(kAnyTransactionID))
                        let list = NSAppleEventDescriptor.list()
                        for (index, url) in urls.enumerated() { list.insert(NSAppleEventDescriptor(fileURL: url), at: index + 1) }
                        event.setParam(list, forKeyword: AEKeyword(keyDirectObject))
                        _ = try? event.sendEvent(options: [.noReply], timeout: 3)
                    } else {
                        let event = NSAppleEventDescriptor(eventClass: AEEventClass(kCoreEventClass), eventID: AEEventID(kAEReopenApplication),
                            targetDescriptor: NSAppleEventDescriptor(processIdentifier: owner), returnID: AEReturnID(kAutoGenerateReturnID), transactionID: AETransactionID(kAnyTransactionID))
                        _ = try? event.sendEvent(options: [.noReply], timeout: 3)
                    }
                }
                Darwin.close(descriptor); exit(0)
            }
            let value = Array("\(getpid())\n".utf8)
            guard ftruncate(descriptor, 0) == 0, Darwin.write(descriptor, value, value.count) == value.count else {
                throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
            }
        } catch {
            let alert = NSAlert(); alert.messageText = "Não foi possível abrir a biblioteca com segurança"
            alert.informativeText = error.localizedDescription; alert.runModal(); exit(1)
        }
    }
}
