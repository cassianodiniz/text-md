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
