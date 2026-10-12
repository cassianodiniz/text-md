import SwiftUI
import TextMDCore

@MainActor
private func metricLabel(_ metric: DocumentMetric, model: AppModel) -> String {
    switch metric {
    case .lines: return "\(model.counts.lines.formatted()) linhas"
    case .words: return "\(model.counts.words.formatted()) palavras"
    case .characters: return "\(model.counts.characters.formatted()) caracteres"
    case .tokens:
        if let count = model.tokenCount { return "\(count.formatted()) tokens" }
        return model.tokensPending ? "… tokens" : "Tokens indisponíveis"
    }
}

struct CounterSummary: View {
    let model: AppModel
    var body: some View {
        if !model.enabledMetrics.isEmpty {
            Text(model.enabledMetrics.map { metricLabel($0, model: model) }.joined(separator: " · "))
                .font(.system(size: 12)).foregroundStyle(.secondary)
                .lineLimit(1).minimumScaleFactor(0.8).monospacedDigit()
                .accessibilityIdentifier("document-counters")
                .accessibilityLabel("Contadores do documento")
                .help("Tokens: \(model.tokenProfile.rawValue), texto completo")
        }
    }
}

struct CountersMenu: View {
    let model: AppModel
    var body: some View {
        Menu {
            ForEach(DocumentMetric.allCases, id: \.self) { metric in
                Toggle(metricLabel(metric, model: model), isOn: Binding(get: { model.enabledMetrics.contains(metric) },
                                                   set: { model.setMetric(metric, enabled: $0) }))
            }
            Divider()
            Button("Mostrar todos") { model.session.metrics = DocumentMetric.allCases; model.persistSession() }
            Button("Ocultar todos") { model.session.metrics = []; model.persistSession() }
            Divider()
            Toggle("Ignorar Markdown", isOn: Binding(get: { model.session.ignoreMarkdown ?? true }, set: { model.setIgnoreMarkdown($0) }))
            Text("Tokens sempre contam o Markdown completo")
            Picker("Perfil de tokens", selection: Binding(get: { model.tokenProfile }, set: { model.setTokenProfile($0) })) {
                Text("OpenAI · o200k_base").tag(TokenProfile.o200k)
                Text("OpenAI · cl100k_base (estudo)").tag(TokenProfile.cl100k)
            }
            if let error = model.tokenError { Text(error) }
        } label: { Text("Contadores") }
    }
}
