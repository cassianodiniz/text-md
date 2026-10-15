import SwiftUI
import AppKit
import TextMDCore

struct WorkspaceView: View {
    @Bindable var model: AppModel
    var body: some View {
        VStack(spacing: 0) {
            DocumentTabs(model: model)
            HSplitView {
                if model.sidebarVisible {
                    SidebarView(model: model).frame(minWidth: 190, idealWidth: model.sidebarWidth, maxWidth: 350)
                        .background(GeometryReader { geometry in Color.clear.preference(key: SidebarWidthKey.self, value: geometry.size.width) })
                        .onPreferenceChange(SidebarWidthKey.self) { value in model.setSidebarWidth(value) }
                        .shadow(color: .black.opacity(0.09), radius: 7, x: 3, y: 0)
                }
                VStack(spacing: 0) {
                    if model.showingTrash { TrashView(model: model) }
                    else if model.showingLibrary { LibraryView(model: model) }
                    else if let draft = model.active {
                        if let state = model.diskStates[draft.id] {
                            HStack(spacing: 10) {
                                Image(systemName: "exclamationmark.triangle")
                                Text(state == .missing ? "Original não encontrado. Seu rascunho está preservado." : "O original mudou por fora. Seu rascunho está preservado.")
                                Spacer()
                                Button("Comparar versões") { model.conflict = ConflictPresentation(id: draft.id, disk: state) }
                            }
                            .font(.system(size: 12)).padding(12).background(Color.orange.opacity(0.10))
                        }
                        if let warning = model.diffWarning {
                            Text(warning).font(.system(size: 12)).foregroundStyle(.orange).padding(8)
                        }
                        MarkdownEditor(draft: draft, model: model)
                            .disabled(model.store == nil)
                            .frame(maxWidth: .infinity, maxHeight: .infinity)
                            .id(draft.id)
                    } else {
                        Color(nsColor: MarkdownStyler.paperColor).frame(maxWidth: .infinity, maxHeight: .infinity)
                    }
                    Divider()
                    statusBar
                }
                .frame(minWidth: 570, maxWidth: .infinity, maxHeight: .infinity)
                .background(Color(nsColor: .textBackgroundColor))
            }
        }
        .frame(minWidth: 820, minHeight: 550)
        .preferredColorScheme(model.session.appearance == .light ? .light : model.session.appearance == .dark ? .dark : nil)
        .toolbar {
            ToolbarItem(placement: .navigation) {
                Button { model.sidebarVisible.toggle(); model.persistSession() } label: { Image(systemName: "sidebar.left") }
                    .help("Mostrar ou ocultar arquivos").accessibilityLabel("Mostrar ou ocultar arquivos")
            }
            ToolbarItemGroup {
                if model.hasDocumentContext { CounterSummary(model: model) }
                Button { model.toggleDiff() } label: {
                    Label("Diff", systemImage: "plus.forwardslash.minus")
                }.disabled(!model.hasDocumentContext)
                    .tint(model.diffEnabled ? .accentColor : .primary)
                    .accessibilityIdentifier("toggle-diff").accessibilityLabel("Mostrar diff")
                    .accessibilityValue(model.diffEnabled ? "Ativo" : "Desativado")
                    .help("Comparar e editar com o original atual")
                Button { model.toggleSource() } label: {
                    Label(model.session.sourceMode ? "Formatado" : "Markdown", systemImage: model.session.sourceMode ? "textformat" : "chevron.left.forwardslash.chevron.right")
                }.disabled(!model.hasDocumentContext || model.diffEnabled).help("Alternar edição formatada e Markdown · ⌘⇧M")
                    .accessibilityIdentifier("toggle-markdown")
                Button { model.showHistory() } label: { Image(systemName: "clock.arrow.circlepath") }
                    .disabled(!model.hasDocumentContext).help("Versões anteriores")
                    .accessibilityLabel("Versões anteriores").accessibilityIdentifier("document-history")
                optionsMenu
            }
        }
        .alert("Não foi possível concluir", isPresented: Binding(get: { model.errorMessage != nil }, set: { if !$0 { model.errorMessage = nil } })) {
            Button("Entendi") { model.errorMessage = nil }
            if model.hasUnsavedMemory { Button("Tentar guardar novamente") { model.errorMessage = nil; model.retryGuarding() } }
        } message: { Text(model.errorMessage ?? "") }
        .sheet(item: $model.conflict) { presentation in ConflictView(model: model, presentation: presentation) }
        .sheet(item: $model.history) { presentation in HistoryView(model: model, draftID: presentation.id) }
        .sheet(isPresented: $model.showClosedDrafts) { ClosedDraftsView(model: model) }
        .onChange(of: model.search) { _, _ in model.searchFiles() }
    }
    private var optionsMenu: some View {
        Menu {
            CountersMenu(model: model)
            Picker("Aparência", selection: Binding(get: { model.session.appearance ?? .system }, set: { model.setAppearance($0) })) {
                Text("Sistema").tag(EditorAppearance.system)
                Text("Clara").tag(EditorAppearance.light)
                Text("Escura").tag(EditorAppearance.dark)
            }
            Toggle("Números de linha", isOn: Binding(get: { model.session.showLineNumbers }, set: { _ in model.toggleLines() }))
                .disabled(!model.session.sourceMode || model.diffEnabled)
            Menu("Zoom") {
                Button("Aumentar · ⌘+") { model.setZoom(Double(model.zoom) + 0.1) }
                Button("Diminuir · ⌘−") { model.setZoom(Double(model.zoom) - 0.1) }
                Button("Tamanho original · ⌘0") { model.setZoom(1) }
                Text("\(Int((model.zoom * 100).rounded()))%")
            }
            Divider()
            Button("Biblioteca") { model.showLibrary() }
            Button("Abrir arquivo…") { model.openPanel() }
            Button("Abrir pasta…") { model.openFolderPanel() }
            Button("Salvar no arquivo") { model.saveActive() }.disabled(!model.hasDocumentContext || model.store == nil)
            Button("Salvar como cópia…") { model.saveActive(asCopy: true) }.disabled(!model.hasDocumentContext || model.store == nil)
            Menu("Formatar") {
                Button("Negrito") { model.format(.bold) }
                Button("Itálico") { model.format(.italic) }
                Button("Título") { model.format(.heading) }
                Button("Link") { model.format(.link) }
                Button("Lista") { model.format(.bullet) }
                Button("Lista de tarefas") { model.format(.task) }
                Button("Código") { model.format(.code) }
                Button("Tabela") { model.format(.table) }
                Button("Imagem…") { model.insertImagePanel() }
            }.disabled(!model.hasDocumentContext)
        } label: { Image(systemName: "ellipsis") }
            .menuIndicator(.hidden)
            .accessibilityLabel("Opções").accessibilityIdentifier("workspace-options")
            .help("Contadores, aparência e outras opções")
    }
    private var statusBar: some View {
        HStack(spacing: 8) {
            if model.showingTrash { Text("Lixeira de rascunhos") }
            else if model.showingLibrary { Text("Biblioteca · rascunhos guardados neste Mac") }
            else if let draft = model.active {
                Image(systemName: model.uncommitted.contains(draft.id) ? "exclamationmark.circle" : "checkmark.circle")
                    .foregroundStyle(model.uncommitted.contains(draft.id) ? .red : .secondary)
                Text(model.uncommitted.contains(draft.id) ? "Ainda não guardado — mantenha o app aberto" : draft.shouldPersist ? "Rascunho guardado" : "Novo rascunho · escreva para guardar")
                if draft.sourcePath != nil {
                    Text("·")
                    Text(model.diskStates[draft.id] != nil ? "Original alterado por fora" : draft.isModified ? "Original intacto" : "Arquivo atualizado")
                }
            } else { Text(model.status) }
            Spacer()
            if model.hasDocumentContext { Text(model.diffEnabled ? "Diff · Markdown" : model.session.sourceMode ? "Markdown" : "Formatado").foregroundStyle(.tertiary) }
        }.font(.system(size: 11)).foregroundStyle(.secondary).padding(.horizontal, 16).padding(.vertical, 9)
    }
}

private struct SidebarWidthKey: PreferenceKey {
    static var defaultValue: CGFloat = 245
    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) { value = nextValue() }
}
