import SwiftUI
import AppKit
import TextMDCore

struct ConflictView: View {
    let model: AppModel
    let presentation: ConflictPresentation
    @Environment(\.dismiss) private var dismiss
    private var draft: Draft? { model.drafts.first { $0.id == presentation.id } }
    private var currentText: String? {
        if case .changed(let text) = presentation.disk { return text }
        return nil
    }
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Image(systemName: "doc.on.doc").font(.system(size: 24)).foregroundStyle(.orange)
                VStack(alignment: .leading, spacing: 5) {
                    Text("O original mudou por fora").font(.system(size: 21, weight: .semibold))
                    Text("Seu rascunho foi preservado. Compare antes de escolher.").foregroundStyle(.secondary)
                }
            }
            HStack(alignment: .top, spacing: 14) {
                ComparisonColumn(title: "ORIGINAL ATUAL", text: currentText ?? unavailableText, other: draft?.content ?? "")
                ComparisonColumn(title: "SEU RASCUNHO", text: draft?.content ?? "", other: currentText ?? "")
            }.frame(maxHeight: .infinity)
            Text("Trechos diferentes aparecem destacados. Nada será sobrescrito automaticamente.")
                .font(.system(size: 11)).foregroundStyle(.secondary)
            HStack {
                Button("Decidir depois") { dismiss() }
                Spacer()
                Button("Descartar rascunho e abrir original…", role: .destructive) {
                    let alert = NSAlert(); alert.messageText = "Abrir a versão atual do original?"
                    alert.informativeText = "Seu rascunho atual ficará nas versões anteriores durante 90 dias. O original não será alterado."
                    alert.addButton(withTitle: "Descartar e abrir original"); alert.addButton(withTitle: "Cancelar")
                    if alert.runModal() == .alertFirstButtonReturn { model.discardAndReload(presentation.id) }
                }.disabled(currentText == nil)
                Button("Salvar rascunho como cópia…") {
                    if let draft { model.savePanel(draft, asCopy: true) }
                }.buttonStyle(.borderedProminent)
            }
        }.padding(24).frame(minWidth: 850, idealWidth: 980, minHeight: 540, idealHeight: 640)
    }
    private var unavailableText: String {
        switch presentation.disk {
        case .missing: return "O original não foi encontrado. Você pode salvar seu rascunho em outro arquivo."
        case .inaccessible(let reason): return "O original não pôde ser lido.\n\n" + reason
        default: return "O original está preservado."
        }
    }
}

struct ComparisonColumn: View {
    let title: String
    let text: String
    let other: String
    private var changedLines: Set<Int> {
        let lines = text.components(separatedBy: "\n")
        let difference = lines.difference(from: other.components(separatedBy: "\n"))
        return Set(difference.compactMap { change in
            if case .insert(let offset, _, _) = change { return offset }
            return nil
        })
    }
    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title).font(.system(size: 10, weight: .semibold)).tracking(1).foregroundStyle(.secondary)
            ScrollView([.horizontal, .vertical]) {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(Array(text.components(separatedBy: "\n").enumerated()), id: \.offset) { index, line in
                        Text(line.isEmpty ? " " : line).font(.system(size: 13, design: .monospaced))
                            .textSelection(.enabled).padding(.horizontal, 8).padding(.vertical, 2)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .background(changedLines.contains(index) ? Color.orange.opacity(0.12) : Color.clear)
                    }
                }.padding(8)
            }.background(Color(nsColor: .textBackgroundColor), in: RoundedRectangle(cornerRadius: 8))
                .overlay(RoundedRectangle(cornerRadius: 8).stroke(.quaternary))
        }.frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct HistoryView: View {
    let model: AppModel
    let draftID: UUID
    @Environment(\.dismiss) private var dismiss
    @ViewState private var revisions: [Revision] = []
    @ViewState private var selection: UUID?
    private var selected: Revision? { revisions.first { $0.id == selection } }
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Versões anteriores").font(.system(size: 22, weight: .semibold))
            Text("Restaurar modifica apenas seu rascunho. O original só muda ao salvar.").foregroundStyle(.secondary)
            if revisions.isEmpty {
                ContentUnavailableView("Ainda não há versões anteriores", systemImage: "clock", description: Text("Versões são guardadas durante a edição e antes de salvar, descartar ou restaurar."))
            } else {
                HSplitView {
                    List(revisions, selection: $selection) { revision in
                        VStack(alignment: .leading, spacing: 4) {
                            Text(Self.date.string(from: revision.createdAt)).font(.system(size: 12, weight: .medium))
                            Text(revision.reason).font(.system(size: 11)).foregroundStyle(.secondary)
                        }.padding(.vertical, 5).tag(revision.id)
                    }.frame(minWidth: 210, idealWidth: 240, maxWidth: 280)
                    ScrollView {
                        Text(selected?.content ?? "Selecione uma versão.").font(.system(size: 14, design: .monospaced))
                            .textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading).padding(16)
                    }.frame(minWidth: 350)
                }
            }
            HStack {
                Text("Versões com mais de 90 dias são removidas automaticamente.").font(.system(size: 11)).foregroundStyle(.secondary)
                Spacer()
                Button("Fechar") { dismiss() }
                Button("Restaurar no rascunho") { if let selected { model.restore(selected) } }
                    .disabled(selected == nil).buttonStyle(.borderedProminent)
            }
        }.padding(24).frame(minWidth: 760, minHeight: 520)
            .onAppear { revisions = model.revisions(for: draftID); selection = revisions.first?.id }
    }
    private static var date: DateFormatter {
        let formatter = DateFormatter(); formatter.locale = Locale(identifier: "pt_BR")
        formatter.timeZone = TimeZone(identifier: "America/Sao_Paulo")
        formatter.dateFormat = "dd/MM/yyyy 'às' HH:mm:ss"; return formatter
    }
}

struct TrashView: View {
    let model: AppModel
    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            HStack {
                Button { model.leaveTrash() } label: { Label("Voltar ao editor", systemImage: "chevron.left") }
                    .keyboardShortcut(.escape, modifiers: [])
                Spacer()
            }
            Text("Lixeira").font(.system(size: 34, design: .serif))
            Text("Rascunhos ficam recuperáveis por 90 dias. Arquivos apagados vão para a Lixeira do Mac. Restaurar aqui recupera somente o rascunho; não recria o arquivo original.")
                .font(.system(size: 13)).foregroundStyle(.secondary)
            if model.trash.isEmpty {
                ContentUnavailableView("A lixeira está vazia", systemImage: "trash")
            } else {
                List(model.trash) { draft in
                    HStack {
                        VStack(alignment: .leading, spacing: 5) {
                            Text(draft.title)
                            Text(String(draft.content.prefix(130))).font(.system(size: 12)).foregroundStyle(.secondary).lineLimit(2)
                        }
                        Spacer()
                        Button("Restaurar rascunho") { model.restoreTrash(draft.id) }
                    }.padding(.vertical, 8)
                }.listStyle(.inset)
            }
            Spacer(minLength: 0)
        }.padding(30).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}


struct ClosedDraftsView: View {
    let model: AppModel
    @Environment(\.dismiss) private var dismiss
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Abas fechadas").font(.system(size: 22, weight: .semibold))
            Text("Os rascunhos continuam na biblioteca. Reabra aqui uma aba fechada.").foregroundStyle(.secondary)
            List(model.hiddenDrafts) { draft in
                HStack {
                    Text(draft.title).lineLimit(1)
                    Spacer()
                    Button("Reabrir") { model.select(draft.id); dismiss() }
                    Button("Apagar", role: .destructive) { model.deleteDraft(draft.id) }
                }
            }
            HStack { Spacer(); Button("Fechar") { dismiss() } }
        }.padding(24).frame(minWidth: 580, minHeight: 380)
    }
}
