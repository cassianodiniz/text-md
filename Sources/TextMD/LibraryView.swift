import SwiftUI
import TextMDCore

struct LibraryView: View {
    let model: AppModel
    private var documents: [Draft] { model.drafts.filter { $0.deletedAt == nil && $0.shouldPersist }.sorted { $0.modifiedAt > $1.modifiedAt } }
    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack {
                Text("Biblioteca").font(.system(size: 30, design: .serif))
                Spacer()
                if model.coordinator?.lastClosedWindow != nil {
                    Button("Restaurar última janela") { model.coordinator?.restoreLastWindow() }
                        .accessibilityIdentifier("restore-last-window")
                }
            }
            if documents.isEmpty {
                Text("Nenhum rascunho guardado.").foregroundStyle(.secondary)
                Spacer()
            } else {
                List(documents) { draft in
                    Button { model.select(draft.id) } label: {
                        VStack(alignment: .leading, spacing: 5) {
                            Text(draft.title).font(.system(size: 15, weight: .medium)).lineLimit(1)
                            Text(draft.sourcePath ?? String(draft.content.prefix(160)))
                                .font(.system(size: 12)).foregroundStyle(.secondary).lineLimit(2)
                        }.frame(maxWidth: .infinity, alignment: .leading).padding(.vertical, 8).contentShape(Rectangle())
                    }.buttonStyle(.plain).accessibilityLabel("Abrir " + draft.title)
                }.listStyle(.plain)
            }
        }.padding(30).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}
