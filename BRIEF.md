# text-md — combinado em 12/10/2026

Aplicativo local para o MacBook de Cassiano, em português, com o visual e a edição
formatada do Typora e a persistência de rascunhos do Notas.

- Sidebar: busca, rascunhos, fixados, pastas reais fixadas, lixeira interna.
- Abas no topo; fonte e espaço em branco baseados nas imagens fornecidas; tema do Mac.
- Rascunhos automáticos, independentes dos arquivos originais; original só muda ao salvar.
- Fechar aba preserva rascunho. Reabrir restaura abas, seleção e posição.
- Conflitos externos: comparação, salvar cópia, descartar rascunho/abrir original, decidir depois.
- Modo Markdown com números de linha opcionais; edição formatada expõe marcações no trecho ativo.
- Command+C copia a seleção como Markdown.
- Títulos, parágrafos, estilos inline, links, listas, citações, código, imagens, tabelas, tarefas e propriedades YAML.
- Busca em rascunhos e Markdown de pastas fixadas; fixar notas.
- Abrir/criar, renomear, mover, mandar arquivo à Lixeira do Mac por ação explícita.
- Desafixar pasta remove só o atalho. Rascunhos acompanham movimentos pelo app.
- Imagens novas internas enquanto rascunho; ao salvar, pasta ao lado do .md e referências relativas.
- Histórico recuperável e lixeira interna: limpeza após 90 dias; ativos e originais nunca expiram.
- Sem login, sincronização ou colaboração. Nunca esvaziar a Lixeira do Mac automaticamente.
- Validar perda de dados, falhas de gravação, conflito, imagem e limpeza; não alegar guardado antes de gravar.

## Decisões de implementação

- SwiftUI para a janela e AppKit para edição e painéis, conforme Build iOS Apps.
- SQLite com transações e confirmação de gravação para recuperação após encerramento inesperado.
- Compilar com Swift disponível neste Mac; Xcode completo não está instalado.
- Projeto em `Skills/text-md`; biblioteca do usuário em Application Support/text-md.


## Revisão aprovada — 12/10/2026, horário de Brasília

Evoluir o app existente com o Whitey instalado e os prints como referência.
A mecânica segue editores nativos do Mac e navegadores; investigar padrões antes
de propor escolhas novas. Tudo permanece neste Mac, sem envio ao GitHub,
inclusive por hooks de commit.

- Ajustar fontes e espaçamentos da lateral, margens entre blocos e títulos e relevo/contorno das abas. Faixa contínua em toda a largura acima da lateral e do editor.
- Propriedades YAML sem quebra de linha, com rolagem horizontal restrita ao bloco.
- Lupa à direita de Fixados; busca aparece ao acioná-la. Seções recolhíveis e estado lembrado.
- Remover “Fixar pasta…” da árvore. Pin à direita ao passar o mouse, selecionar ou usar teclado; pin também no caminho atual. Fixar/desafixar não abre nem apaga a pasta.
- Menu lateral alterna Pastas e Documentos/rascunhos. Pastas mostra todos os arquivos, permite entrar e subir e acompanha o documento selecionado. Não nativos cinza, clicáveis e abertos no app padrão.
- Clique simples usa a aba atual; Command+clique e links Markdown locais abrem nova aba. Arquivo já aberto traz sua aba e janela existentes, sem duplicação, preservando edições.
- Várias janelas dentro de uma única instância; biblioteca/tema/zoom/fixados compartilhados e abas/navegação independentes.
- Arraste reorganiza, transfere entre janelas e cria janela ao soltar fora. Fechar a última aba fecha aquela janela. Command+Q restaura janelas e abas abertas.
- Command+T novo rascunho, Command+N nova janela, Command+W fechar aba, Command+Shift+W fechar janela, Command+Shift+T reabrir aba, Control+Tab e inverso trocar abas, Command+1–8 escolher e Command+9 última.
- Command+O abrir arquivo, Command+Shift+O abrir pasta; manter salvar/cópia, busca local/global, negrito/itálico/link, cópia Markdown e desfazer/refazer. Command+Shift+L alterna lateral.
- Command+mais/menos/0 ajusta/restaura zoom dos documentos em todas as abas/janelas, lembrado ao reabrir. Controles mantêm o tamanho.
- Preservar rascunhos separados dos originais, conflitos, histórico, imagens, Diff e contadores offline sem tempo de leitura.
- Validar com bibliotecas e documentos temporários próprios; instalar no aplicativo existente sem perder os dados do usuário.
