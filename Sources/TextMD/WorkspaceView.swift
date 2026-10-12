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
                    SidebarView(model: model).frame(minWidth: 190, idealWidth: 245, maxWidth: 350)
                        .shadow(color: .black.opacity(0.09), radius: 7, x: 3, y: 0)
                }
                VStack(spacing: 0) {
                    if model.showingTrash { TrashView(model: model) }
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
                            .id("editor-host")
                    } else {
                        ContentUnavailableView {
                            Label("Sua próxima ideia começa aqui", systemImage: "square.and.pencil")
                        } description: {
                            Text("Crie um rascunho ou abra um arquivo. Você pode fechar sem salvar no original.")
                        } actions: {
                            Button("Novo rascunho") { model.newDraft() }.buttonStyle(.borderedProminent)
                            Button("Abrir arquivo…") { model.openPanel() }
            Button("Abrir pasta…") { model.openFolderPanel() }
                        }.frame(maxWidth: .infinity, maxHeight: .infinity)
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
                CounterSummary(model: model)
                Button { model.toggleDiff() } label: {
                    Label("Diff", systemImage: "plus.forwardslash.minus")
                }.disabled(model.active == nil)
                    .tint(model.diffEnabled ? .accentColor : .primary)
                    .accessibilityIdentifier("toggle-diff").accessibilityLabel("Mostrar diff")
                    .accessibilityValue(model.diffEnabled ? "Ativo" : "Desativado")
                    .help("Comparar e editar com o original atual")
                Button { model.toggleSource() } label: {
                    Label(model.session.sourceMode ? "Formatado" : "Markdown", systemImage: model.session.sourceMode ? "textformat" : "chevron.left.forwardslash.chevron.right")
                }.disabled(model.diffEnabled).help("Alternar edição formatada e Markdown · ⌘⇧M")
                    .accessibilityIdentifier("toggle-markdown")
                Button { model.showHistory() } label: { Image(systemName: "clock.arrow.circlepath") }
                    .disabled(model.active == nil).help("Versões anteriores")
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
        .onChange(of: model.search) { _, _ in model.searchFiles() }
        .onChange(of: model.active?.sourcePath) { _, path in
            if let path, model.currentFolder != URL(fileURLWithPath: path).deletingLastPathComponent().path {
                model.followActiveFolder(); model.persistSession()
            }
        }
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
            Button("Abrir arquivo…") { model.openPanel() }
            Button("Abrir pasta…") { model.openFolderPanel() }
            Button("Salvar no arquivo") { model.saveActive() }.disabled(model.active == nil || model.store == nil)
            Button("Salvar como cópia…") { model.saveActive(asCopy: true) }.disabled(model.active == nil || model.store == nil)
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
            }.disabled(model.active == nil)
        } label: { Image(systemName: "ellipsis") }
            .menuIndicator(.hidden)
            .accessibilityLabel("Opções").accessibilityIdentifier("workspace-options")
            .help("Contadores, aparência e outras opções")
    }
    private var statusBar: some View {
        HStack(spacing: 8) {
            if let draft = model.active {
                Image(systemName: model.uncommitted.contains(draft.id) ? "exclamationmark.circle" : "checkmark.circle")
                    .foregroundStyle(model.uncommitted.contains(draft.id) ? .red : .secondary)
                Text(model.uncommitted.contains(draft.id) ? "Ainda não guardado — mantenha o app aberto" : draft.shouldPersist ? "Rascunho guardado" : "Novo rascunho · escreva para guardar")
                if draft.sourcePath != nil {
                    Text("·")
                    Text(model.diskStates[draft.id] != nil ? "Original alterado por fora" : draft.isModified ? "Original intacto" : "Arquivo atualizado")
                }
            } else { Text(model.status) }
            Spacer()
            Text(model.diffEnabled ? "Diff · Markdown" : model.session.sourceMode ? "Markdown" : "Formatado").foregroundStyle(.tertiary)
        }.font(.system(size: 11)).foregroundStyle(.secondary).padding(.horizontal, 16).padding(.vertical, 9)
    }
}

extension Notification.Name { static let textmdFocusSearch = Notification.Name("textmd.focusSearch") }
