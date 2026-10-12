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
                        } label: { Image(systemName: "ellipsis").frame(width: 24, height: 24) }
                            .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                            .accessibilityLabel("Navegação lateral").accessibilityIdentifier("sidebar-options")
                    }.padding(.horizontal, 14).padding(.top, 18).padding(.bottom, 5)
                    if model.sectionExpanded(model.sidebarMode.rawValue) {
                        if model.sidebarMode == .folders { folderContents }
                        else {
                            ForEach(model.visibleDrafts.filter { !$0.pinned }) { draft in draftRow(draft) }
                            if model.visibleDrafts.filter({ !$0.pinned }).isEmpty {
                                Text("Seus rascunhos ficam guardados aqui.").font(.system(size: 12)).foregroundStyle(.secondary).padding(14)
                            }
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
        Button { model.select(draft.id, newTab: (NSApp.currentEvent?.modifierFlags ?? NSEvent.modifierFlags).contains(.command)) } label: {
            HStack(spacing: 7) {
                Image(systemName: draft.pinned ? "pin.fill" : draft.sourcePath == nil ? "note.text" : "doc.text").font(.system(size: 12)).foregroundStyle(.secondary)
                Text(draft.title).lineLimit(1)
                Spacer(minLength: 0)
                if model.uncommitted.contains(draft.id) { Image(systemName: "exclamationmark.circle").foregroundStyle(.red) }
                else if draft.isModified && draft.sourcePath != nil { Circle().fill(.secondary).frame(width: 4, height: 4) }
            }.font(sidebarFont).padding(.horizontal, 14).frame(height: 32)
                .frame(maxWidth: .infinity, alignment: .leading).contentShape(Rectangle())
                .background(model.session.selectedID == draft.id && !model.showingTrash ? Color.primary.opacity(0.07) : .clear)
        }.buttonStyle(.plain).accessibilityLabel(draft.title).accessibilityIdentifier("draft:" + draft.id.uuidString)
            .contextMenu {
                Button(draft.pinned ? "Desafixar nota" : "Fixar nota") { model.pin(draft.id) }
                Button("Versões anteriores") { model.history = HistoryPresentation(id: draft.id) }
                Button("Enviar rascunho à lixeira interna", role: .destructive) { model.trashDraft(draft.id) }
                if let path = draft.sourcePath { Button("Mostrar no Finder") { NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)]) } }
            }
    }
    private func fileRow(_ node: FileNode) -> some View {
        let selected = model.active?.sourcePath.map { FileService.documentKey(URL(fileURLWithPath: $0)) == FileService.documentKey(node.url) } == true
        return Button { model.open(node.url, newTab: (NSApp.currentEvent?.modifierFlags ?? NSEvent.modifierFlags).contains(.command)) } label: {
            HStack(spacing: 7) {
                Image(systemName: node.isEditable ? "doc.text" : "doc").font(.system(size: 12))
                Text(node.name).lineLimit(1).truncationMode(.middle)
                Spacer(minLength: 0)
            }.font(sidebarFont).foregroundStyle(node.isEditable ? Color.primary : Color.secondary)
                .padding(.horizontal, 14).frame(height: 32).frame(maxWidth: .infinity, alignment: .leading)
                .background(selected ? Color.primary.opacity(0.07) : .clear).contentShape(Rectangle())
        }.buttonStyle(.plain).accessibilityLabel("Arquivo " + node.name + (node.isEditable ? "" : " · abre em outro aplicativo"))
            .accessibilityIdentifier("file:" + node.url.path)
            .contextMenu {
                Button("Abrir") { model.open(node.url, newTab: false) }
                if node.isEditable { Button("Abrir em nova aba") { model.open(node.url) } }
                Button("Renomear…") { model.rename(node.url) }
                Button("Mover…") { model.movePanel(node.url) }
                Button("Mostrar no Finder") { NSWorkspace.shared.activateFileViewerSelecting([node.url]) }
                Divider()
                Button("Enviar arquivo à Lixeira do Mac…", role: .destructive) { model.trashFile(node.url) }
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
                }.padding(.leading, showsIcon ? 14 : 3).padding(.trailing, 32).frame(height: 32)
                    .frame(maxWidth: .infinity, alignment: .leading).contentShape(Rectangle())
            }.buttonStyle(.plain).focused($focused)
                .accessibilityLabel("Pasta " + title).accessibilityIdentifier("folder:" + url.path)
            Button { model.togglePinnedFolder(url) } label: {
                Image(systemName: model.folderIsPinned(url) ? "pin.fill" : "pin").font(.system(size: 12))
                    .frame(width: 28, height: 28).contentShape(Rectangle())
            }.buttonStyle(.plain).foregroundStyle(.secondary).padding(.trailing, 6)
                .opacity(pinVisible ? 1 : 0).allowsHitTesting(pinVisible).accessibilityHidden(!pinVisible)
                .accessibilityLabel(model.folderIsPinned(url) ? "Desafixar pasta" : "Fixar pasta")
                .accessibilityIdentifier("pin-folder:" + url.path)
        }.background(selected ? Color.primary.opacity(0.07) : .clear)
            .onHover { hovered = $0 }
            .help(url.path)
            .contextMenu {
                Button("Abrir pasta") { model.navigateFolder(url) }
                Button(model.folderIsPinned(url) ? "Desafixar pasta" : "Fixar pasta") { model.togglePinnedFolder(url) }
                Button("Novo documento aqui…") { model.createFile(in: url) }
                Button("Mostrar no Finder") { NSWorkspace.shared.activateFileViewerSelecting([url]) }
            }
    }
}
