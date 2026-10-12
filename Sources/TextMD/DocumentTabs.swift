import SwiftUI
import AppKit
import TextMDCore
import UniformTypeIdentifiers

/// One shared track spanning the window; many tabs scroll instead of shrinking indefinitely.
struct DocumentTabs: View {
    let model: AppModel
    var body: some View {
        GeometryReader { geometry in
            let trackWidth = max(0, geometry.size.width - 44)
            let tabWidth = max(140, (trackWidth - 4) / CGFloat(max(1, model.tabs.count)))
            HStack(spacing: 6) {
                ScrollViewReader { proxy in
                    ScrollView(.horizontal) {
                        HStack(spacing: 0) {
                            ForEach(model.tabs) { draft in
                                tab(draft).frame(width: tabWidth, height: 28).id(draft.id)
                            }
                        }.padding(2)
                    }
                    .scrollIndicators(.hidden)
                    .onAppear { revealSelection(using: proxy) }
                    .onChange(of: model.session.selectedID) { _, _ in revealSelection(using: proxy) }
                    .onChange(of: model.tabs.count) { _, _ in revealSelection(using: proxy) }
                }
                .background(Color.primary.opacity(0.065), in: RoundedRectangle(cornerRadius: 15))
                Button { model.newDraft() } label: {
                    Image(systemName: "plus").font(.system(size: 17, weight: .regular))
                        .frame(width: 38, height: 38).contentShape(Rectangle())
                }.buttonStyle(.plain).foregroundStyle(.secondary)
                    .accessibilityLabel("Novo rascunho").accessibilityIdentifier("new-draft-tab")
                    .help("Novo rascunho · ⌘T")
            }
        }
        .frame(height: 34).padding(.horizontal, 8).padding(.vertical, 5)
        .background(Color(nsColor: .windowBackgroundColor))
        .onDrop(of: ["com.cassiano.text-md.tab"], isTargeted: nil) { providers in
            guard let provider = providers.first else { return false }
            provider.loadDataRepresentation(forTypeIdentifier: "com.cassiano.text-md.tab") { data, _ in
                guard let data, let raw = String(data: data, encoding: .utf8), let id = UUID(uuidString: raw) else { return }
                Task { @MainActor in model.coordinator?.moveTab(id, to: model) }
            }
            return true
        }
    }

    private func revealSelection(using proxy: ScrollViewProxy) {
        if let id = model.session.selectedID { proxy.scrollTo(id, anchor: .center) }
    }

    private func tab(_ draft: Draft) -> some View {
        let selected = model.session.selectedID == draft.id
        return ZStack(alignment: .trailing) {
            Button { model.select(draft.id) } label: {
                HStack(spacing: 5) {
                    Text(draft.title).lineLimit(1).truncationMode(.middle)
                    if draft.isModified && draft.sourcePath != nil {
                        Circle().fill(.secondary).frame(width: 5, height: 5)
                    }
                }
                .padding(.horizontal, 32)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .contentShape(Rectangle())
            }.buttonStyle(.plain)
                .accessibilityIdentifier("tab:" + draft.id.uuidString)
                .accessibilityLabel(draft.title)
                .accessibilityValue(selected ? "Selecionada" : "")
                .overlay(TabDragHandle(model: model, id: draft.id, title: draft.title).padding(.trailing, 31))
            Button { model.closeTab(draft.id) } label: {
                Image(systemName: "xmark").font(.system(size: 9))
                    .frame(width: 28, height: 28).contentShape(Rectangle())
            }.buttonStyle(.plain).foregroundStyle(.secondary).padding(.trailing, 3)
                .accessibilityIdentifier("close-tab:" + draft.id.uuidString)
                .accessibilityLabel("Fechar " + draft.title)
                .help("Fechar aba; rascunho fica guardado")
        }
        .accessibilityElement(children: .contain)
        .font(.system(size: 12))
        .foregroundStyle(selected ? .primary : .secondary)
        .contextMenu {
            Button("Fechar aba") { model.closeTab(draft.id) }
            Button("Mover para nova janela") { model.coordinator?.detachTab(draft.id) }
            if let coordinator = model.coordinator {
                ForEach(coordinator.models.filter { $0 !== model }, id: \.windowID) { target in
                    Button("Mover para janela · " + (target.active?.title ?? "text-md")) { coordinator.moveTab(draft.id, to: target) }
                }
            }
        }
        .background {
            if selected {
                RoundedRectangle(cornerRadius: 13)
                    .fill(Color(nsColor: .textBackgroundColor))
                    .overlay(RoundedRectangle(cornerRadius: 13).strokeBorder(Color.primary.opacity(0.08), lineWidth: 0.5))
                    .shadow(color: .black.opacity(0.045), radius: 1.5, y: 0.5)
            }
        }
    }
}
