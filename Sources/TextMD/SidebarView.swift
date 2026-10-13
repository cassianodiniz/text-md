import SwiftUI
import AppKit
import TextMDCore

struct SidebarView: View {
    @Bindable var model: AppModel
    @FocusState private var searchFocused: Bool
    private var sidebarFont: Font { Font(MarkdownStyler.serif(14)) }
    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                sectionButton("FIXADOS", key: "pinned")
                Spacer(minLength: 0)
                Button {
                    model.searchVisible.toggle()
                    if !model.searchVisible { model.search = "" }
                } label: { Image(systemName: "magnifyingglass").frame(width: 24, height: 28) }
                    .buttonStyle(.plain).accessibilityLabel("Buscar nos textos").accessibilityIdentifier("toggle-search")
            }.padding(.horizontal, 14).padding(.top, 10)
            if model.searchVisible {
                HStack(spacing: 5) {
                    TextField("Buscar nos textos", text: $model.search).textFieldStyle(.plain)
                        .focused($searchFocused).accessibilityIdentifier("sidebar-search")
                    Button { model.search = ""; model.searchVisible = false } label: { Image(systemName: "xmark").font(.system(size: 10)) }
                        .buttonStyle(.plain).accessibilityLabel("Fechar busca")
                }.font(.system(size: 12)).padding(8).background(.quaternary.opacity(0.2), in: RoundedRectangle(cornerRadius: 4)).padding(.horizontal, 12)
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    if model.sectionExpanded("pinned") {
                        ForEach(model.session.folders) { folder in
                            folderRow(URL(fileURLWithPath: folder.path))
                        }
                        ForEach(model.visibleDrafts.filter(\.pinned)) { draft in draftRow(draft) }
                    }
                    HStack(spacing: 6) {
                        sectionButton(model.sidebarMode == .folders ? "PASTA ATUAL" : "DOCUMENTOS", key: model.sidebarMode.rawValue)
                        Spacer(minLength: 0)
                        Menu {
                            Button("Pastas") { model.setSidebarMode(.folders) }
                            Button("Documentos / rascunhos") { model.setSidebarMode(.documents) }
                            Divider()
                            Button("Abrir pasta…") { model.openFolderPanel() }
                            Button("Recuperar fechados…") { model.showClosedDrafts = true }
                        } label: { Image(systemName: "ellipsis").frame(width: 24, height: 24) }
                            .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                            .accessibilityLabel("Navegação lateral").accessibilityIdentifier("sidebar-options")
                    }.padding(.horizontal, 14).padding(.top, 18).padding(.bottom, 5)
                    if model.sectionExpanded(model.sidebarMode.rawValue) {
                        if model.sidebarMode == .folders { folderContents }
                        else {
                            ForEach(model.visibleDrafts.filter { !$0.pinned }) { draft in draftRow(draft) }
                        }
                    }
                    if !model.search.isEmpty {
                        Text("NOS ARQUIVOS").font(.system(size: 10)).foregroundStyle(.secondary).padding(14)
                        if model.searchRunning { ProgressView().controlSize(.small).padding(12) }
                        ForEach(model.fileResults) { result in
                            fileRow(FileNode(url: URL(fileURLWithPath: result.path), isDirectory: false))
                        }
                        if let warning = model.searchWarning { Text(warning).font(.system(size: 11)).foregroundStyle(.orange).padding(12) }
                    }
                }.frame(maxWidth: .infinity, alignment: .leading).padding(.bottom, 12)
            }
            Divider()
            HStack {
                Button { model.flushEditor(); model.showingTrash = true } label: { Label("Lixeira", systemImage: "trash") }
                    .buttonStyle(.plain).foregroundStyle(model.showingTrash ? .primary : .secondary)
                Spacer()
                Button { model.newDraft() } label: { Image(systemName: "square.and.pencil") }
                    .buttonStyle(.plain).help("Novo rascunho · ⌘T").accessibilityLabel("Novo rascunho")
            }.padding(14).font(.system(size: 12))
        }
        .background(Color(nsColor: MarkdownStyler.paperColor))
        .onChange(of: model.searchVisible) { _, visible in if visible { searchFocused = true } }
        .onReceive(NotificationCenter.default.publisher(for: .textmdFocusSearch)) { notice in
            guard let id = notice.object as? UUID, id == model.windowID else { return }
            model.sidebarVisible = true; model.searchVisible = true; searchFocused = true
        }
    }
    private func sectionButton(_ title: String, key: String) -> some View {
        Button { model.toggleSection(key) } label: {
            HStack(spacing: 5) {
                Text(title).font(Font(MarkdownStyler.serif(13)))
                Image(systemName: model.sectionExpanded(key) ? "chevron.down" : "chevron.right").font(.system(size: 9))
            }.frame(height: 30).contentShape(Rectangle())
        }.buttonStyle(.plain).foregroundStyle(.secondary)
            .accessibilityLabel(title).accessibilityValue(model.sectionExpanded(key) ? "Expandido" : "Recolhido")
            .accessibilityIdentifier("section:" + key)
    }
    @ViewBuilder private var folderContents: some View {
        if let path = model.currentFolder {
            let url = URL(fileURLWithPath: path)
            HStack(spacing: 3) {
                Button { model.navigateFolder(url.deletingLastPathComponent()) } label: { Image(systemName: "arrow.up").frame(width: 24, height: 28) }
                    .buttonStyle(.plain).disabled(url.path == "/")
                    .help("Subir uma pasta").accessibilityLabel("Subir uma pasta").accessibilityIdentifier("folder-up")
                FolderSidebarRow(model: model, url: url, title: url.path == "/" ? "/" : url.deletingLastPathComponent().lastPathComponent + " / " + url.lastPathComponent, selected: true, showsIcon: false)
            }.padding(.leading, 9)
            if let error = model.folderErrors[path] { Text(error).font(.system(size: 11)).foregroundStyle(.orange).padding(12) }
            ForEach(model.children[path] ?? []) { node in
                if node.isDirectory { folderRow(node.url) } else { fileRow(node) }
            }
            if model.children[path]?.isEmpty == true && model.folderErrors[path] == nil {
                Text("Esta pasta está vazia.").font(.system(size: 12)).foregroundStyle(.secondary).padding(14)
            }
        } else {
            Button("Abrir pasta…") { model.openFolderPanel() }.buttonStyle(.plain).padding(14)
        }
    }
    private func folderRow(_ url: URL) -> some View {
        FolderSidebarRow(model: model, url: url, title: url.lastPathComponent,
            selected: model.sidebarMode == .folders && model.currentFolder.map { FileService.documentKey(URL(fileURLWithPath: $0)) == FileService.documentKey(url) } == true)
    }
    private func draftRow(_ draft: Draft) -> some View {
        SidebarDocumentRow(title: draft.title, icon: draft.sourcePath == nil ? "note.text" : "doc.text",
            identifier: "draft:" + draft.id.uuidString, selected: model.session.selectedID == draft.id && !model.showingTrash,
            pinned: draft.pinned, muted: false,
            open: { model.select(draft.id, newTab: $0) },
            rename: { model.renameDraft(draft.id, to: $0) }, pin: { model.pin(draft.id) }) {
            Menu("Abrir em") {
                Button("Esta aba") { model.select(draft.id, newTab: false) }
                Button("Nova aba") { model.select(draft.id) }
                Button("Nova janela") { model.openDraftInNewWindow(draft.id) }
            }
            LabelledPin(pinned: draft.pinned) { model.pin(draft.id) }
            Button { model.moveDraftPanel(draft.id) } label: { Label("Mover para…", systemImage: "folder") }
            Divider()
            Button(role: .destructive) { model.deleteDraft(draft.id) } label: { Label("Apagar", systemImage: "trash") }
        }
    }
    private func fileRow(_ node: FileNode) -> some View {
        let selected = model.active?.sourcePath.map { FileService.documentKey(URL(fileURLWithPath: $0)) == FileService.documentKey(node.url) } == true
        let pinned = model.draftForFile(node.url)?.pinned == true
        return SidebarDocumentRow(title: node.name, icon: node.isEditable ? "doc.text" : "doc",
            identifier: "file:" + node.url.path, selected: selected, pinned: pinned, muted: !node.isEditable,
            open: { model.open(node.url, newTab: $0) },
            rename: { model.renameFile(node.url, to: $0) }, pin: node.isEditable ? { model.toggleFilePin(node.url) } : nil) {
            Menu("Abrir em") {
                if node.isEditable {
                    Button("Esta aba") { model.open(node.url, newTab: false) }
                    Button("Nova aba") { model.open(node.url) }
                    Button("Nova janela") { model.openFileInNewWindow(node.url) }
                } else {
                    Button("Aplicativo padrão") { model.open(node.url) }
                }
            }
            if node.isEditable { LabelledPin(pinned: pinned) { model.toggleFilePin(node.url) } }
            Button { model.movePanel(node.url) } label: { Label("Mover para…", systemImage: "folder") }
            Divider()
            Button(role: .destructive) { model.trashFile(node.url) } label: { Label("Apagar", systemImage: "trash") }
        }
    }

}

private struct FolderSidebarRow: View {
    let model: AppModel
    let url: URL
    let title: String
    let selected: Bool
    var showsIcon = true
    @ViewState private var hovered = false
    @FocusState private var focused: Bool
    private var pinVisible: Bool { hovered || focused || selected }
    var body: some View {
        ZStack(alignment: .trailing) {
            Button { model.navigateFolder(url) } label: {
                HStack(spacing: 7) {
                    if showsIcon { Image(systemName: "folder.fill").font(.system(size: 12)).foregroundStyle(.secondary) }
                    Text(title).font(Font(MarkdownStyler.serif(14))).lineLimit(1).truncationMode(.middle)
                    Spacer(minLength: 0)
                }.padding(.leading, showsIcon ? 14 : 3).padding(.trailing, 62).frame(height: 32)
                    .frame(maxWidth: .infinity, alignment: .leading).contentShape(Rectangle())
            }.buttonStyle(.plain).focused($focused)
                .accessibilityLabel("Pasta " + title).accessibilityIdentifier("folder:" + url.path)
            Button { model.togglePinnedFolder(url) } label: {
                Image(systemName: model.folderIsPinned(url) ? "pin.fill" : "pin").font(.system(size: 12))
                    .frame(width: 28, height: 28).contentShape(Rectangle())
            }.buttonStyle(.plain).foregroundStyle(.secondary).padding(.trailing, 33)
                .opacity(pinVisible ? 1 : 0).allowsHitTesting(pinVisible).accessibilityHidden(!pinVisible)
                .accessibilityLabel(model.folderIsPinned(url) ? "Desafixar pasta" : "Fixar pasta")
                .accessibilityIdentifier("pin-folder:" + url.path)
            Menu {
                Menu("Abrir em") {
                    Button("Nesta janela") { model.navigateFolder(url) }
                    Button("Nova janela") { model.coordinator?.createWindow(in: url) }
                }
                LabelledPin(pinned: model.folderIsPinned(url)) { model.togglePinnedFolder(url) }
                Button { model.movePanel(url) } label: { Label("Mover para…", systemImage: "folder") }
                Divider()
                Button(role: .destructive) { model.trashFile(url) } label: { Label("Apagar", systemImage: "trash") }
            } label: { Image(systemName: "ellipsis").frame(width: 24, height: 28) }
                .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize().padding(.trailing, 5)
                .accessibilityLabel("Opções de " + title).accessibilityIdentifier("options:folder:" + url.path)
        }.background(selected ? Color.primary.opacity(0.07) : .clear)
            .onHover { hovered = $0 }
            .help(url.path)
            .contextMenu {
                Menu("Abrir em") {
                    Button("Nesta janela") { model.navigateFolder(url) }
                    Button("Nova janela") { model.coordinator?.createWindow(in: url) }
                }
                LabelledPin(pinned: model.folderIsPinned(url)) { model.togglePinnedFolder(url) }
                Button { model.movePanel(url) } label: { Label("Mover para…", systemImage: "folder") }
                Divider()
                Button(role: .destructive) { model.trashFile(url) } label: { Label("Apagar", systemImage: "trash") }
            }
    }
}


private struct LabelledPin: View {
    let pinned: Bool
    let action: () -> Void
    var body: some View {
        Button(action: action) { Label(pinned ? "Desafixar" : "Fixar", systemImage: pinned ? "pin.slash" : "pin") }
    }
}

/// The row menu is visible without requiring a right click. Rename edits the
/// label itself and commits on Return or focus loss; Escape cancels it.
private struct SidebarDocumentRow<Actions: View>: View {
    let title: String
    let icon: String
    let identifier: String
    let selected: Bool
    let pinned: Bool
    let muted: Bool
    let open: (Bool) -> Void
    let rename: (String) -> Bool
    let pin: (() -> Void)?
    @ViewBuilder let actions: () -> Actions
    @ViewState private var renaming = false
    @ViewState private var name = ""
    @ViewState private var hovered = false
    @FocusState private var nameFocused: Bool
    var body: some View {
        HStack(spacing: 0) {
            if renaming {
                HStack(spacing: 7) {
                    Image(systemName: icon).font(.system(size: 12)).foregroundStyle(.secondary)
                    TextField("Nome", text: $name).textFieldStyle(.plain).focused($nameFocused)
                        .onSubmit { finishRename() }
                        .onExitCommand { renaming = false }
                        .accessibilityIdentifier("rename:" + identifier)
                        .task { nameFocused = true }
                }.padding(.leading, 14).frame(maxWidth: .infinity, alignment: .leading)
            } else {
                Button { open((NSApp.currentEvent?.modifierFlags ?? NSEvent.modifierFlags).contains(.command)) } label: {
                    HStack(spacing: 7) {
                        Image(systemName: icon).font(.system(size: 12)).foregroundStyle(.secondary)
                        Text(title).lineLimit(1).truncationMode(.middle)
                        Spacer(minLength: 0)
                    }.padding(.leading, 14).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                        .contentShape(Rectangle())
                }.buttonStyle(.plain).accessibilityIdentifier(identifier)
                    .accessibilityLabel(title).help("Duplo clique para renomear")
                    .overlay(SidebarRowClick(open: open, rename: { name = title; renaming = true }))
            }
            if let pin {
                Button(action: pin) {
                    Image(systemName: pinned ? "pin.fill" : "pin").font(.system(size: 12)).frame(width: 24, height: 28)
                }.buttonStyle(.plain).foregroundStyle(.secondary)
                    .opacity(hovered || selected || pinned ? 1 : 0)
                    .allowsHitTesting(hovered || selected || pinned).accessibilityHidden(!(hovered || selected || pinned))
                    .accessibilityLabel(pinned ? "Desafixar" : "Fixar").accessibilityIdentifier("pin:" + identifier)
                    .disabled(renaming)
            }
            Menu(content: actions) { Image(systemName: "ellipsis").frame(width: 24, height: 28) }
                .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                .accessibilityLabel("Opções de " + title).accessibilityIdentifier("options:" + identifier)
                .disabled(renaming).padding(.trailing, 5)
        }
        .font(Font(MarkdownStyler.serif(14))).foregroundStyle(muted ? Color.secondary : Color.primary)
        .frame(height: 32).frame(maxWidth: .infinity, alignment: .leading)
        .background(selected ? Color.primary.opacity(0.07) : .clear)
        .onHover { hovered = $0 }
        .onChange(of: nameFocused) { _, focused in if !focused && renaming { finishRename() } }
        .contextMenu { actions() }
    }
    private func finishRename() {
        if rename(name) { renaming = false }
        else { nameFocused = true }
    }
}
