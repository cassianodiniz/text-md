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
    private var zoomMonitor: Any?
    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApplication.shared.setActivationPolicy(.regular); NSApplication.shared.activate(ignoringOtherApps: true)
        // The menu lists one action; the shifted plus key is an equivalent alias.
        zoomMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
            let handled = MainActor.assumeIsolated {
                if event.modifierFlags.intersection(.deviceIndependentFlagsMask).subtracting([.shift, .capsLock]) == .command,
                   event.characters == "+", let model = WindowCoordinator.shared.active {
                    model.setZoom(Double(model.zoom) + 0.1); return true
                }
                return false
            }
            return handled ? nil : event
        }
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
            Button("Salvar") { model?.saveActive() }.keyboardShortcut("s").disabled(model?.hasDocumentContext != true)
            Button("Salvar como cópia…") { model?.saveActive(asCopy: true) }.keyboardShortcut("s", modifiers: [.command, .shift]).disabled(model?.hasDocumentContext != true)
        }
        CommandMenu("Formatar") {
          Group {
            Button("Negrito") { model?.format(.bold) }.keyboardShortcut("b")
            Button("Itálico") { model?.format(.italic) }.keyboardShortcut("i")
            Button("Título") { model?.format(.heading) }.keyboardShortcut("h", modifiers: [.command, .option])
            Button("Link") { model?.format(.link) }.keyboardShortcut("k")
            Button("Lista") { model?.format(.bullet) }
            Button("Lista de tarefas") { model?.format(.task) }
            Button("Código") { model?.format(.code) }
            Button("Tabela") { model?.format(.table) }
            Button("Imagem…") { model?.insertImagePanel() }
          }.disabled(model?.hasDocumentContext != true)
        }
        CommandMenu("Texto") {
            Button(model?.session.sourceMode == true ? "Edição formatada" : "Mostrar Markdown") { model?.toggleSource() }
                .keyboardShortcut("m", modifiers: [.command, .shift]).disabled(model?.hasDocumentContext != true || model?.diffEnabled == true)
            Button("Números de linha") { model?.toggleLines() }.disabled(model?.session.sourceMode != true || model?.diffEnabled == true)
            Button(model?.diffEnabled == true ? "Ocultar diff" : "Mostrar diff") { model?.toggleDiff() }.disabled(model?.hasDocumentContext != true)
            Divider()
            Button("Buscar nos textos") { model?.focusSearch() }.keyboardShortcut("f", modifiers: [.command, .shift])
            Button("Buscar neste texto") { model?.findInDocument() }.keyboardShortcut("f").disabled(model?.hasDocumentContext != true)
            Button("Versões anteriores") { model?.showHistory() }.disabled(model?.hasDocumentContext != true)
        }
        CommandMenu("Abas") {
            Button("Fechar aba") {
                if let model, model.hasDocumentContext, let id = model.active?.id { model.closeTab(id) }
                else { model?.nativeWindow?.performClose(nil) }
            }.keyboardShortcut("w")
            Button("Fechar janela") { model?.nativeWindow?.performClose(nil) }.keyboardShortcut("w", modifiers: [.command, .shift])
            Button("Reabrir última aba ou janela fechada") { coordinator.reopenClosed() }
                .keyboardShortcut("t", modifiers: [.command, .shift]).disabled(coordinator.closedItems.isEmpty)
            Button("Restaurar última janela") { coordinator.restoreLastWindow() }.disabled(coordinator.lastClosedWindow == nil)
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
            Button("Aumentar zoom") { model?.setZoom(Double(model?.zoom ?? 1) + 0.1) }.keyboardShortcut("=")
            Button("Diminuir zoom") { model?.setZoom(Double(model?.zoom ?? 1) - 0.1) }.keyboardShortcut("-")
            Button("Tamanho original") { model?.setZoom(1) }.keyboardShortcut("0")
            Divider()
            Button("Biblioteca") { model?.showLibrary() }
            Button("Mostrar ou ocultar lateral") { model?.sidebarVisible.toggle(); model?.persistSession() }.keyboardShortcut("l", modifiers: [.command, .shift])
        }
    }
}
