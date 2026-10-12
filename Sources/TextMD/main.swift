import SwiftUI
import AppKit
import TextMDCore

@main @MainActor
struct TextMDApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    init() { SingleInstance.prepare(); NSWindow.allowsAutomaticWindowTabbing = false }
    var body: some Scene {
        WindowGroup("text-md", id: "workspace") { WorkspaceScene().handlesExternalEvents(preferring: ["*"], allowing: ["*"]) }
            .handlesExternalEvents(matching: [])
            .defaultSize(width: 1180, height: 780)
            .windowToolbarStyle(.unifiedCompact)
            .commands { EditorCommands(coordinator: .shared) }
    }
}

struct WorkspaceScene: View {
    @Environment(\.openWindow) private var openWindow
    @ViewState private var model: AppModel?
    var body: some View {
        Group {
            if let model { WorkspaceView(model: model) }
            else { Color.clear.frame(minWidth: 820, minHeight: 550) }
        }.background(WindowAttachment { window in
            let coordinator = WindowCoordinator.shared
            coordinator.openWindowAction = { openWindow(id: "workspace") }
            let instance = model ?? coordinator.claimWindow()
            model = instance
            coordinator.attach(instance, to: window)
        })
    }
}

struct WindowAttachment: NSViewRepresentable {
    let attach: (NSWindow) -> Void
    func makeNSView(context: Context) -> AttachmentView { let view = AttachmentView(); view.attach = attach; return view }
    func updateNSView(_ view: AttachmentView, context: Context) { view.attach = attach; view.attachIfNeeded() }
    final class AttachmentView: NSView {
        var attach: ((NSWindow) -> Void)?
        private weak var attachedWindow: NSWindow?
        override func viewDidMoveToWindow() { super.viewDidMoveToWindow(); attachIfNeeded() }
        func attachIfNeeded() {
            guard let window, attachedWindow !== window else { return }
            attachedWindow = window
            DispatchQueue.main.async { [weak self, weak window] in if let window { self?.attach?(window) } }
        }
    }
}

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApplication.shared.setActivationPolicy(.regular); NSApplication.shared.activate(ignoringOtherApps: true)
    }
    func application(_ application: NSApplication, open urls: [URL]) { WindowCoordinator.shared.open(urls) }
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        WindowCoordinator.shared.reopenApplication(); return false
    }
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        let coordinator = WindowCoordinator.shared; coordinator.flushAll()
        if !coordinator.library.uncommitted.isEmpty || coordinator.library.sessionWriteFailed {
            let alert = NSAlert(); alert.alertStyle = .critical
            alert.messageText = "Algumas edições ainda não foram guardadas"
            alert.informativeText = "Mantenha o text-md aberto e tente guardar novamente ou salve uma cópia."
            alert.addButton(withTitle: "Continuar no aplicativo"); alert.runModal()
            return .terminateCancel
        }
        coordinator.terminating = true; return .terminateNow
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }
}

struct EditorCommands: Commands {
    let coordinator: WindowCoordinator
    private var model: AppModel? { coordinator.active }
    var body: some Commands {
        CommandGroup(replacing: .newItem) {
            Button("Novo rascunho") { if let model { model.newDraft() } else { coordinator.createWindow() } }.keyboardShortcut("t")
            Button("Nova janela") { coordinator.createWindow() }.keyboardShortcut("n")
            Button("Abrir arquivo…") { coordinator.openPanel(folder: false) }.keyboardShortcut("o")
            Button("Abrir pasta…") { coordinator.openPanel(folder: true) }.keyboardShortcut("o", modifiers: [.command, .shift])
        }
        CommandGroup(replacing: .saveItem) {
            Button("Salvar") { model?.saveActive() }.keyboardShortcut("s").disabled(model?.active == nil)
            Button("Salvar como cópia…") { model?.saveActive(asCopy: true) }.keyboardShortcut("s", modifiers: [.command, .shift]).disabled(model?.active == nil)
        }
        CommandGroup(replacing: .undoRedo) {
            Button("Desfazer") { model?.undo() }.keyboardShortcut("z").disabled(model?.canUndo != true)
            Button("Refazer") { model?.redo() }.keyboardShortcut("z", modifiers: [.command, .shift]).disabled(model?.canRedo != true)
        }
        CommandMenu("Formatar") {
            Button("Negrito") { model?.format(.bold) }.keyboardShortcut("b")
            Button("Itálico") { model?.format(.italic) }.keyboardShortcut("i")
            Button("Título") { model?.format(.heading) }.keyboardShortcut("h", modifiers: [.command, .option])
            Button("Link") { model?.format(.link) }.keyboardShortcut("k")
            Button("Lista") { model?.format(.bullet) }
            Button("Lista de tarefas") { model?.format(.task) }
            Button("Código") { model?.format(.code) }
            Button("Tabela") { model?.format(.table) }
            Button("Imagem…") { model?.insertImagePanel() }
        }
        CommandMenu("Texto") {
            Button(model?.session.sourceMode == true ? "Edição formatada" : "Mostrar Markdown") { model?.toggleSource() }
                .keyboardShortcut("m", modifiers: [.command, .shift]).disabled(model?.diffEnabled == true)
            Button("Números de linha") { model?.toggleLines() }.disabled(model?.session.sourceMode != true || model?.diffEnabled == true)
            Button(model?.diffEnabled == true ? "Ocultar diff" : "Mostrar diff") { model?.toggleDiff() }.disabled(model?.active == nil)
            Divider()
            Button("Buscar nos textos") {
                model?.sidebarVisible = true; model?.searchVisible = true
                NotificationCenter.default.post(name: .textmdFocusSearch, object: model?.windowID)
            }.keyboardShortcut("f", modifiers: [.command, .shift])
            Button("Buscar neste texto") { model?.editor?.showFind() }.keyboardShortcut("f")
            Button("Versões anteriores") { model?.showHistory() }.disabled(model?.active == nil)
        }
        CommandMenu("Abas") {
            Button("Fechar aba") { if let model, let id = model.active?.id { model.closeTab(id) } }.keyboardShortcut("w")
            Button("Fechar janela") { model?.nativeWindow?.performClose(nil) }.keyboardShortcut("w", modifiers: [.command, .shift])
            Button("Reabrir aba fechada") { coordinator.reopenClosed() }.keyboardShortcut("t", modifiers: [.command, .shift])
            Divider()
            Button("Próxima aba") { model?.switchTab(1) }.keyboardShortcut(.tab, modifiers: [.control])
            Button("Aba anterior") { model?.switchTab(-1) }.keyboardShortcut(.tab, modifiers: [.control, .shift])
            ForEach(1..<10, id: \.self) { index in
                Button(index == 9 ? "Última aba" : "Aba \(index)") { model?.selectTab(index) }.keyboardShortcut(KeyEquivalent(Character(String(index))))
            }
            Divider()
            Button("Mover aba para nova janela") { if let id = model?.active?.id { coordinator.detachTab(id) } }
        }
        CommandMenu("Exibir") {
            Button("Aumentar zoom") { model?.setZoom(Double(model?.zoom ?? 1) + 0.1) }.keyboardShortcut("+")
            Button("Aumentar zoom") { model?.setZoom(Double(model?.zoom ?? 1) + 0.1) }.keyboardShortcut("=")
            Button("Diminuir zoom") { model?.setZoom(Double(model?.zoom ?? 1) - 0.1) }.keyboardShortcut("-")
            Button("Tamanho original") { model?.setZoom(1) }.keyboardShortcut("0")
            Divider()
            Button("Mostrar ou ocultar lateral") { model?.sidebarVisible.toggle(); model?.persistSession() }.keyboardShortcut("l", modifiers: [.command, .shift])
        }
    }
}
