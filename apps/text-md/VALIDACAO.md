# Verificação do text-md

12/10/2026, horário de Brasília. MacBook, macOS 26.7.1, Apple Silicon.

## Automatizada

18 testes de comportamento passaram com Swift Testing:

- Reabrir a biblioteca recupera texto, abas, seleção e preferências.
- Editar o rascunho deixa o original intacto; salvar atualiza sua referência.
- Alteração externa é detectada pelo conteúdo, inclusive com a mesma data de modificação.
- Conflito bloqueia substituição; salvar cópia preserva os dois textos.
- Original ausente não é recriado silenciosamente.
- Destino existente e biblioteca somente leitura informam falhas sem perder a última gravação.
- Restaurar histórico preserva uma versão do trabalho substituído.
- Limpeza remove apenas lixeira interna e histórico com mais de 90 dias; mantém o limite exato, ativos e originais.
- Imagens novas ficam privadas até salvar; depois têm referência relativa e cópia ao lado do documento.
- Imagem interna ausente bloqueia substituição do original; caminho de imagem inválido é rejeitado.
- Abrir e salvar um arquivo sem edição preserva bytes UTF-8, BOM e CRLF.
- Mover documento preserva bytes e suas imagens locais relativas.
- Mover não substitui documento ou imagem de destino existente.
- Salvar cópia em outra pasta mantém imagens relativas acessíveis.

Os testes usam diretórios temporários próprios. A coordenação de arquivos do
macOS exige executar os testes fora da restrição do executor; sob essa restrição,
o sistema devolve erro 512 antes de executar a escrita. O teste de controle fora
da restrição confirmou que isso não era uma falha de salvamento do app.

## Na janela real

Biblioteca temporária, separada dos dados do usuário:

- Escrever um rascunho e conferir a gravação local.
- Encerrar o processo à força e reabrir: texto recuperado integralmente.
- Command+C no modo formatado: títulos, negrito e tarefas copiados como Markdown.
- Alternar para Markdown, ativar e ocultar números de linha.
- Conferir visualmente títulos, estilos, tarefas, tabela e propriedades YAML.
- Abrir arquivo existente, editar sem salvar e conferir original intacto.
- Modificar original externamente: comparação abriu com as diferenças destacadas.
- Salvar rascunho como cópia pelo diálogo: original externo e rascunho preservados em arquivos distintos.
- Inserir imagem pelo painel, conferir prévia e salvar: pasta de imagens e referência relativa criadas.
- Fixar pasta real temporária e encontrar um documento por seu conteúdo.
- Abrir o histórico e acionar restauração: arquivo original permaneceu intacto.
- Na compilação de uso, trocar de abas, desfazer na primeira, refazer e conferir a segunda intacta; o teste está em `scripts/ui_smoke.py`.

O controle de interface CUA falhou ao iniciar nesta sessão. A verificação da janela
foi feita com a acessibilidade do macOS e capturas exclusivamente da janela do app.

## Limites da verificação

- Não houve espera real de 90 dias: a regra foi verificada com datas controladas.
- Não foi simulado desligamento físico ou falha de disco; houve encerramento forçado do processo e falha de escrita controlada.
- Não houve teste em outros Macs nem verificação de todas as extensões do ecossistema Markdown.
- Não houve sincronização ou colaboração, que estão fora do combinado.

## Correções durante a conferência

- Selecionar o armazenamento de estado tradicional resolve a ausência do plugin da nova macro State no SDK instalado.
- Cache local e configuração do compilador evitam caminhos bloqueados do executor.
- Preservação explícita da marca BOM evita uma alteração invisível em documentos.
- Ajuste do tamanho inicial da área de rolagem evita deslocar o texto para fora da janela.
- Tabelas são desenhadas com células; as marcações permanecem no armazenamento e aparecem ao editar a linha.
- A régua de linhas desenha somente os números; preencher seu retângulo de atualização cobria o editor.
- O modelo principal é criado uma vez para evitar rascunhos vazios duplicados na inicialização.
- Os comandos de desfazer/refazer consultam o histórico da aba, em vez do histórico ausente no contexto padrão do menu SwiftUI. A regressão foi reproduzida e verificada na janela real.

## Revisão 0.2.0 — 12/10/2026

Os 26 testes atuais passaram. Além da cobertura anterior:

- Novos vazios e espaços não geram registros; escrever e depois apagar mantém a nota e o histórico.
- A migração coloca somente vazios antigos indevidos na lixeira recuperável. Preserva arquivos vazios e evidências de escrita anterior, inclusive histórico já expirado.
- Identidade do arquivo reconhece caminhos equivalentes sem mudar a base das referências de imagem.
- 142 textos, nos dois perfis, totalizando 284 contagens, coincidem com `tiktoken` oficial 0.14.0. Incluem português, japonês, árabe, emoji composto, CRLF, Markdown, marcadores especiais tratados como texto literal, repetições longas e exemplos determinísticos variados.
- Ignorar Markdown preserva exemplos literais dentro de código; linhas não dependem da largura; caracteres compostos são uma unidade.
- Diff cobre substituição, remoção no final, remoção de tudo, adição em nota nova e ausência de mudanças, com posições UTF-16.
- A sessão antiga é lida sem perder dados; tema, contadores, perfil e diff persistem.

`scripts/ui_smoke.py` confirmou na janela que abrir/fechar vazios não os guarda,
e que desfazer/refazer continuam independentes por aba.

`scripts/ui_revision.py` confirmou com biblioteca temporária e processo próprio:

- Cliques nas bordas direita e esquerda da pasta e na borda do arquivo.
- Reabrir o mesmo arquivo mantém o mesmo rascunho, inclusive em caminhos equivalentes do macOS.
- Texto pode ser editado no diff; original permanece intacto; Command+C copia somente o Markdown atual.
- Menu sem tempo de leitura, alternância de Ignorar Markdown e escolha do tema escuro.
- Remover todo o conteúdo mantém as saídas vermelhas visíveis e todos os contadores em zero.
- Encerrar à força e reabrir recupera conteúdo e preferências.

Capturas foram inspecionadas nas aparências clara e escura. O desenho usa os
parâmetros tipográficos do Whitey instalado como referência dos prints. Não se
afirma igualdade de pixels de todas as extensões Markdown ou de todos os tamanhos
de janela com o mecanismo WebKit do Typora.

A conferência detectou e corrigiu o uso do retângulo da linha, que incluía o
espaço reservado ao diff: usar o retângulo do glifo evita cobrir o texto anterior.
Também corrigiu aliases de caminho e contagem de links literais em código.
Os scripts de UI precisam do primeiro plano para as interações; aguardam os
estados efetivos e preservam todos os formatos da área de transferência.

### Fontes de tokens

- [OpenAI — Counting tokens](https://developers.openai.com/api/docs/guides/token-counting): texto local e distinção entre texto do arquivo e estrutura de uma requisição.
- [OpenAI Cookbook — tiktoken](https://developers.openai.com/cookbook/examples/how_to_count_tokens_with_tiktoken): perfis e contagem por codificação. A página é arquivada; os vocabulários incorporados são verificados pelo hash do pacote 0.14.0.
- Petrov et al., *Language Model Tokenizers Introduce Unfairness Between Languages* (2023), PDF indicado pelo usuário: diferença entre idiomas e perfis, sem aplicar um multiplicador fixo ao português.

O app usa apenas dados locais e não faz chamadas à OpenAI. Os dados e a licença
MIT estão em `Sources/TextMDCore/Resources`; o gerador de referência é exclusivo
do desenvolvimento, sem dependência de Python na instalação.


## Revisão 0.2.1 — 12/10/2026, horário de Brasília

A barra nativa da janela reúne as contagens passivas, Diff, Markdown, versões e
Opções. Aparência, seleção de contadores, números de linha e ações menos usadas
ficam no menu de três pontos. As abas dividem toda a largura acima da lateral;
com muitas abas, a faixa rola e acompanha a seleção. A seleção de rascunhos na
lateral usa sombreamento retangular de ponta a ponta.

Os 26 testes de comportamento passaram. `scripts/ui_layout.py` verificou em uma
biblioteca temporária própria a posição das contagens e ações acima das abas, a
largura igual das abas, seleção de documentos, menu Opções, Ignorar Markdown,
tema escuro, Diff, Markdown e abertura de versões. Capturas da janela de teste
foram inspecionadas nos temas claro e escuro. Esse teste não digita nos textos
nem acessa a biblioteca ou a área de transferência do usuário.

`scripts/ui_smoke.py` também passou: rascunhos vazios continuam temporários,
desfazer/refazer permanecem independentes por aba e os textos ficam guardados.

Os scripts anteriores agora localizam o editor e as abas por seus identificadores
de acessibilidade, sem depender da posição dos grupos na janela.


## Revisão 0.3.0 — 12/10/2026, horário de Brasília

Os 31 testes de comportamento passaram. Os cinco novos testes cobrem a
restauração de várias janelas, seleção válida, identidade única dos arquivos,
transferência de abas sem cópias, sessão antiga, abas vazias temporárias e
referências locais relativas/absolutas com espaços e fragmentos.

`scripts/ui_browser.py` passou com biblioteca, arquivos e processo temporários:

- Propriedades YAML sem quebra de linha; rolagem horizontal percorre a linha
  longa sem deslocar o título ou o corpo do texto.
- Clique em referência Markdown local abre a referência numa aba selecionada;
  fechar devolve à origem e à sua pasta.
- Recolher Fixados preserva os pins; fixar/desfixar não muda a pasta atual.
- Entrar e subir pastas, mostrar também arquivos externos, clique simples na
  aba atual e Command+clique numa nova aba.
- Command+N cria outra janela; zoom e tamanho original são compartilhados.
- Executar novamente o aplicativo com um arquivo já aberto termina a segunda
  instância e ativa a janela existente sem criar uma janela ou aba extra.
- Arraste real entre janelas e para fora cria/move a aba conservando sua
  identidade e conteúdo.
- Fechar a última aba fecha a janela; Command+Shift+T recupera o rascunho.
- Tema, contadores e todas as janelas/abas guardadas retornam após Command+Q.
- O arquivo original permaneceu com os mesmos bytes.

`scripts/ui_revision.py` passou novamente na compilação final: edição no diff,
cópia Markdown sem trechos removidos, remoção total do texto, tema, contadores e
recuperação após interromper somente o processo de teste. A área de transferência
foi preservada em todos os seus formatos.

`scripts/ui_smoke.py` também passou na compilação final: vazios continuam
temporários e desfazer/refazer permanecem independentes por aba.

Capturas próprias foram inspecionadas nas aparências clara e escura. O desenho
continua usando o Whitey como referência; não se afirma igualdade de pixels em
todos os documentos ou compatibilidade com todas as extensões Markdown.

Durante a conferência, foram corrigidos o controle horizontal inicialmente
desabilitado e a janela vazia extra que o encaminhamento padrão de eventos do
SwiftUI criava ao reabrir um arquivo. A automação de várias janelas identifica
cada janela pelo seu ID e a localiza novamente depois de trazê-la à frente, pois
a ordem das janelas muda nesse momento. Nenhum documento do usuário foi usado
nesses testes.


`scripts/ui_layout.py` passou na versão final: posição das contagens e ações,
largura igual das abas acima da lateral, seleção, Opções, temas, Diff, Markdown
e abertura do histórico. O SwiftUI emitiu um aviso de ciclo ao abrir o histórico;
o painel abriu e a verificação terminou sem falha observada.

A instalação local substituiu a versão anterior após seu encerramento normal.
O aplicativo anterior e a biblioteca completa foram copiados para uma pasta de
segurança; os arquivos da biblioteca foram conferidos por hash antes da troca.
A versão 0.3.0 foi reaberta no caminho habitual. O código fica somente neste Mac,
conforme solicitado.


## Repositório próprio — 12/10/2026, horário de Brasília

Por solicitação posterior do usuário, o projeto foi movido para
`~/Projetos/Apple Smart Apps/text-md` e preparado para publicação no repositório
público `cassianodiniz/text-md`. A autorização anterior de manter somente no Mac
foi substituída por esse pedido. Caches, aplicativo compilado e biblioteca pessoal
não fazem parte do repositório. Nenhum teste de interface foi executado na mudança.


## Distribuição 0.3.0 — 12/10/2026, horário de Brasília

Licença MIT do projeto e avisos de tokenização preservados no código e no pacote.
A release usa a compilação 0.3.0 já existente, para arm64, com mínimo macOS 14.
Foram conferidos versão, arquitetura, permissões do executável, assinatura ad hoc
e integridade da imagem de disco. Não foi iniciado o app nem executado teste de
interface na preparação desta release. Não se afirma teste em Intel ou em outros
Macs. A distribuição não tem Developer ID/notarização; isso está documentado nas
instruções de primeira abertura.


## Revisão 0.3.1 — 13/10/2026, horário de Brasília

Origem do problema: a lateral incluía todas as notas guardadas em memória,
inclusive as fechadas e as temporárias vazias; as ações dependiam de botão direito.
O editor desenhava explicitamente dois textos de sugestão quando vazio.

Correção: menu visível por item; edição inline do nome com duplo clique; nome
próprio para notas sem arquivo; renomeação real sem mudar o Markdown; fechamento
com estado oculto persistente e recuperação separada; migração de fechados antigos
sem apagar seu conteúdo. Recarregar uma alteração externa preserva esse estado.
Os gestos distinguem clique simples de duplo antes de abrir o documento e capturam
Command no próprio evento, inclusive para documentos de outras janelas.

Compilação release e 35 testes de dados passaram. Os quatro novos casos usam
somente diretórios temporários: fechar/reabrir biblioteca, migração, preservação
após alteração externa, renomear mantendo BOM/CRLF, colisões de destino, alteração
apenas de maiúsculas/minúsculas, títulos YAML e vazios fora da lateral.

Não foi aberto o aplicativo para testes, nem executados scripts de interface ou
capturas de tela. O comportamento visual do gesto e dos menus não foi conferido
com cliques reais nesta revisão, conforme a preferência do usuário.

A instalação 0.3.1 foi substituída após encerramento normal da versão anterior.
Foi feito backup do aplicativo e da biblioteca; os hashes da biblioteca
coincidiram antes e depois da troca. O app novo não foi iniciado para evitar
tomar o foco da tela. DMG e ZIP preservam a MIT e os avisos de terceiros.


## Revisão 0.3.2 — 14/10/2026, horário de Brasília

Correções dos 18 achados descritas individualmente em [AUDIT_FIXES.md](AUDIT_FIXES.md).
Os 43 testes de dados/Markdown e os 8 testes do modelo/editor passaram (51 no total).
A compilação de testes inclui o aplicativo para validar também os comandos e o
ciclo de vida do editor, usando AppKit sem criar janelas. Os testes não iniciam
`NSApplication.run`, não usam a biblioteca pessoal e não executam `ui_*`.

Cobertura adicional: Markdown literal e contexto de código; fences longos,
Setext, tabelas, referências e Unicode; exportação de imagens reais em vez de
prefixos literais; dependências ao mover/copiar; link simbólico; fechamento
atômico com falha exclusiva de sessão; colisão entre rascunhos no destino;
restauração sem adotar substituto no caminho antigo; histórico nativo de
desfazer/refazer ao entrar/sair da Lixeira; mídia órfã e referências codificadas;
contexto e cancelamento da busca; compatibilidade da largura com sessão antiga.

Como nas verificações anteriores do núcleo, as operações coordenadas de arquivo
exigem sair da restrição do executor. Uma tentativa dentro dela recebeu erro
Cocoa 512; a mesma suíte passou fora dela com pastas descartáveis. Isso não abriu
a interface nem ampliou os testes para arquivos pessoais.

Os testes executados não comprovam os gestos completos de foco, arraste do
divisor, navegação por teclado, VoiceOver ou aparência da janela. Esses limites
estão identificados por achado; não foram executados testes visuais a pedido
do usuário. O aplicativo não será iniciado ao terminar a instalação.


A compilação release 0.3.2 (build 6, arm64) terminou com sucesso. Foram conferidos
Info.plist, assinatura ad hoc, arquitetura, licenças incorporadas e ausência de
bundles de teste no pacote. O compilador avisou sobre Sendable em um monitor de
NSEvent (o projeto usa modo de linguagem Swift 5) e caminhos ausentes opcionais
do Command Line Tools; nenhum deles impediu a compilação ou a assinatura.

Instalação concluída em 14/10/2026 às 22:23, horário de Brasília, em
`/Users/cassianodiniz/Applications/text-md.app`. A saída ignorada de build do
projeto também foi atualizada. Backup consistente SQLite dos seis rascunhos,
assets e aplicativo anterior em
`/Users/cassianodiniz/Library/Application Support/text-md-backups/2026-10-14-221624`.
Os quatro arquivos da biblioteca ficaram com hashes idênticos antes e depois da
troca. Nenhum documento original foi escrito; a biblioteca atual não tem originais
externos associados. O app não estava em execução e não foi iniciado, encerrado
ou ativado. Os logs e o relatório da instalação acompanham o backup.

A instalação local foi concluída antes da publicação. A distribuição 0.3.2
reutiliza essa compilação, sem repetir testes de interface ou acessar os
rascunhos pessoais. As versões públicas anteriores permanecem sem alteração.


### Pacotes da release 0.3.2

DMG e ZIP gerados da compilação instalada, sem iniciar o app. A verificação do
DMG confirmou sua integridade. O ZIP foi extraído em pasta temporária: versão,
arquitetura arm64, permissão do executável, assinatura ad hoc, licenças e bytes
do executável coincidem com a compilação instalada. Pacote sem bundles de teste
ou biblioteca pessoal; SHA-256 conferido para ambos os downloads.

Os textos de licença das dependências, originalmente somente leitura, são
copiados para uma pasta nova no pacote temporário; os avisos acompanham tanto o
app quanto a raiz do DMG. O código de funcionamento do aplicativo não foi alterado
na preparação da release; os 51 testes anteriores continuam sendo a evidência
correspondente à compilação distribuída, sem nova execução de testes de interface.


## Ciclo de vida revisado — 15/10/2026, horário de Brasília

Contrato aprovado: Command+W fecha só a aba e conserva o rascunho na biblioteca;
Dock sem janelas abre Biblioteca e oferece restaurar a última janela.
Compilação 0.3.2 build 7 para substituir a preparação local build 6, ainda não publicada.

67 verificações passaram sem interface: 43 do núcleo e 24 do modelo/editor.
Os novos casos cobrem última aba real (com pré-condição explícita de uma aba),
zero janelas, reabertura lógica do Dock, grupos de janela após reiniciar,
seleção/ordem/navegação, fechamento de fundo, histórico após encerrar sem janelas,
reservas pendentes e transferência da última aba. Também cobrem cópias antigas
independentes, referências de imagem preservadas, recarga externa com editor em
cache, backup de versão retido após falha/retry e renomeação para caminho reservado
por outro rascunho com original ausente. Testes anteriores de visibilidade foram
atualizados para o novo contrato, mantendo a prova de recuperação do conteúdo.

A revisão independente foi estática. As transições foram verificadas usando os
modelos reais com SQLite e arquivos temporários; não houve NSWindow, clique no
Dock, inicialização do app ou execução de scripts ui_*. Isso não comprova o
comportamento visual completo do menu, divisor, foco e eventos reais do sistema.
A restauração de geometria de janela pelo SwiftUI continua sem nova afirmação;
não se acrescentou outro mecanismo sem evidência de falha.

Antes da instalação foi feito backup SQLite consistente, com integrity_check OK,
dos oito registros atuais, além da versão anterior do aplicativo. A sessão aberta
do usuário não será encerrada pelo atualizador; o novo executável será utilizado
quando o usuário encerrar normalmente e abrir o app novamente.


Instalação da build 7 concluída sem encerrar a sessão existente. A cópia anterior
e a biblioteca estão em `~/Library/Application Support/text-md-backups/2026-10-15-085853`.
O atualizador não abriu o app nem gravou na biblioteca pessoal. Como o processo
anterior estava rodando, a nova compilação passa a ser usada na próxima abertura
após encerramento normal.

DMG e ZIP finais da build 7 conferidos: integridade do DMG, SHA-256 de ambos,
extração do ZIP, Info.plist, assinatura ad hoc, licenças e executável idêntico ao
instalado. Pacotes sem biblioteca pessoal e sem bundles de teste. Os pacotes
locais anteriores da build 6 não foram publicados e foram separados da versão
final para não serem enviados por engano.
