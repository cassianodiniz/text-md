# text-md

Editor Markdown para macOS, feito para escrever posts e editar skills com
rascunhos persistentes. A aparência segue o Typora; o trabalho fica guardado
localmente, como nas notas, e o original só muda ao salvar.

## Baixar e instalar

Baixe o **[text-md 0.3.1 para Mac Apple Silicon](https://github.com/cassianodiniz/text-md/releases/tag/v0.3.1)**.
O pacote requer macOS 14 ou posterior e um Mac com chip Apple (M1 ou posterior);
não contém uma versão para Macs Intel.

1. Baixe `text-md-0.3.1-macos-arm64.dmg` e abra o arquivo.
2. Arraste **text-md.app** para **Applications / Aplicativos**.
3. Ejete a imagem e abra o app na pasta Aplicativos.

Ao atualizar, encerre normalmente a versão anterior antes de substituir o app.
Seus rascunhos ficam na biblioteca do usuário e não são apagados pela troca do app.

Esta distribuição tem assinatura local ad hoc, sem certificado Developer ID
ou notarização da Apple. Se o macOS bloquear a primeira abertura por não
identificar o desenvolvedor, tente abrir o app e depois use **Ajustes do Sistema
→ Privacidade e Segurança → Abrir Mesmo Assim**, conforme a
[orientação oficial da Apple](https://support.apple.com/en-us/102445).

A release também oferece um ZIP com o aplicativo e `SHA256SUMS.txt` para
conferência dos arquivos. Não é necessário compilar o código para instalar.

## Abrir e escrever

Abra o **text-md.app** na pasta Aplicativos do seu usuário. Não é preciso usar o
terminal, entrar em uma conta ou manter conexão com a internet.

- **⌘T:** novo rascunho, sem exigir nome ou pasta.
- **⌘N:** nova janela.
- **⌘⇧T:** reabrir a última aba fechada.
- **⌘O:** abrir um arquivo existente; **⌘⇧O:** abrir uma pasta.
- **⌘S:** salvar no original, ou escolher um arquivo para um texto novo.
- **⌘⇧S:** salvar como cópia.
- **⌘⇧M:** alternar edição formatada e Markdown.
- **Texto → Números de linha:** mostrar/ocultar linhas no modo Markdown.
- **⌘C:** copiar a seleção com as marcações de Markdown.
- **⌘B / ⌘I / ⌘K:** negrito, itálico e link.
- **⌘F:** buscar no texto atual; **⌘⇧F:** buscar nos rascunhos e nas pastas fixadas.
- **⌘W:** fechar a aba; **⌘⇧W:** fechar a janela. Rascunhos que receberam texto continuam disponíveis na biblioteca.
- **Control+Tab / Control+Shift+Tab:** próxima aba/anterior. **⌘1–8 / ⌘9:** escolher uma aba/a última.
- **⌘+ / ⌘− / ⌘0:** aumentar, diminuir e restaurar o zoom dos documentos.
- **⌘⇧L:** mostrar ou ocultar a lateral.

Um rascunho novo vazio, ou contendo somente espaços e quebras de linha, é
temporário. Ele começa a ser guardado quando você escreve conteúdo. Se apagar
tudo depois de escrever, a nota e suas versões continuam preservadas.

Cada item da lateral tem um menu **⋯** com **Abrir em**, **Fixar/Desafixar**,
**Mover para…** e **Apagar**. O pin também pode ser acionado diretamente na linha.
**Mover para…** abre o seletor de pasta; um rascunho ainda sem arquivo passa a ser
um documento Markdown naquela pasta.

Dê **duplo clique no nome** para renomear na própria lateral. **Enter** confirma,
**Esc** cancela. Em arquivos existentes, isso altera o nome real no disco e o nome
da aba; a extensão é preservada quando você não digita outra. Rascunhos sem arquivo
têm um nome próprio, que será usado ao salvá-los. O conteúdo não é reescrito pelo
ato de renomear. Propriedades YAML não viram o título do rascunho.

**Fechar uma aba retira o documento da lateral**, preservando seu texto para
recuperação. Ele não volta automaticamente após reiniciar o app. Use
**⋯ da seção → Recuperar fechados…** ou **⌘⇧T** para reabri-lo explicitamente.
Encerrar o aplicativo com **⌘Q** continua preservando as abas que você deixou abertas.
Rascunhos novos vazios não aparecem na lateral, e o editor começa em branco,
sem sugestões de escrita.

**Apagar** um arquivo ou pasta os envia à Lixeira do Mac, com confirmação; seus
rascunhos saem da lateral e ficam recuperáveis na lixeira interna por 90 dias.
Apagar uma nota sem arquivo afeta somente a biblioteca do app. Desafixar remove
somente o atalho. **Fixados** aparece primeiro; os cabeçalhos recolhem e expandem
as seções, lembrando seu estado ao reabrir.

## Pastas, abas e janelas

Em **⋯ na lateral**, alterne entre **Pastas** e **Documentos/rascunhos**. O modo
Pastas mostra o conteúdo da pasta atual e uma seta para subir. A lateral acompanha
a pasta do documento selecionado. A lupa ao lado de Fixados revela a busca.

Clique simples abre um documento na aba atual, mantendo o rascunho anterior na
biblioteca. **Command+clique** abre uma nova aba. Links locais para arquivos `.md`
abrem uma aba selecionada. Se o documento já estiver aberto, o app traz sua janela
e seleciona a aba existente, inclusive para caminhos equivalentes.

Todos os arquivos são listados. Os não editáveis ficam em cinza mais discreto e
abrem no aplicativo padrão do Mac. Markdown, `.txt` e textos como `LICENSE` e
`README` sem extensão são editáveis no text-md.

Cada janela tem suas próprias abas e navegação. Rascunhos, fixados, tema,
contadores e zoom são compartilhados. Arraste uma aba para reorganizar, movê-la
para outra janela ou criar uma janela ao soltá-la fora. Também há ações no menu
da aba e em **Abas**. Fechar a última aba fecha a janela; **⌘Q** encerra o app.
Abrir o aplicativo novamente usa a mesma instância e biblioteca.

O zoom fica lembrado e modifica apenas o conteúdo dos documentos, mantendo o
tamanho dos controles. O bloco cinza de propriedades não quebra as linhas;
use a rolagem horizontal no próprio bloco para percorrer linhas longas.

## Aparência e contadores

As abas dividem toda a largura da janela em uma faixa contínua, acima da lateral
e do editor. **Diff**, **Markdown** e **Versões anteriores** ficam na barra
superior. O menu **⋯** reúne contadores, tema, números de linha, abertura,
salvamento e formatação; os atalhos de teclado continuam disponíveis.

O menu **⋯**, na barra superior, oferece **Aparência → Sistema, Clara e
Escura**. Sistema é o padrão. A apresentação formatada segue o tema Whitey do
Typora dos prints: fonte serifada, títulos centralizados, sublinha no segundo
nível e propriedades YAML em uma faixa cinza clara.

As contagens aparecem na barra superior, sem seletor. Em **⋯ → Contadores**,
escolha **linhas, palavras, caracteres e tokens**, em qualquer combinação. Não há tempo de leitura. A opção
**Ignorar Markdown** muda palavras e caracteres; linhas continuam contando as
quebras do arquivo, e tokens sempre incluem todo o Markdown. Caracteres contam
símbolos visíveis como um emoji composto como uma unidade, incluindo espaços.

Os perfis **OpenAI · o200k_base** e **OpenAI · cl100k_base (estudo)** usam os
vocabulários oficiais incorporados no aplicativo. A contagem funciona offline,
sem chave de API ou envio de textos. É a contagem do texto do arquivo no perfil
escolhido, sem o contexto adicional de uma conversa ou requisição de API.

## Diff durante a escrita

O botão **Diff** compara seu rascunho com o arquivo original atual, incluindo
mudanças feitas por outros aplicativos. Ele mostra Markdown, entradas em verde
e saídas em vermelho, e permite continuar escrevendo. Desligar volta ao modo
anterior. Em notas sem arquivo, todo o texto aparece como adicionado até salvar.

Trechos removidos são uma camada de comparação: não entram no rascunho, na
contagem ou no **⌘C**. Se o original estiver ausente ou inacessível, o app informa
que a comparação não está disponível e mantém o rascunho preservado.

## Recuperação

Cada alteração é confirmada na biblioteca local antes de indicar **Rascunho
guardado**. Fechar o aplicativo restaura o trabalho na próxima abertura, incluindo
abas, cursor e posição. Se a biblioteca não conseguir guardar alguma edição, o
app informa a falha e impede um encerramento normal que perderia essas edições.

Se o original mudar por fora enquanto houver um rascunho diferente, o app mostra
uma comparação. Você pode salvar uma cópia, descartar o rascunho e abrir a versão
atual, ou decidir depois. O rascunho descartado também fica no histórico.

O botão do relógio abre **Versões anteriores**. Restaurar modifica apenas o
rascunho. Lixeira interna e versões com mais de **90 dias** são limpas
automaticamente; ativos e originais não expiram. A Lixeira do Mac não é esvaziada
pelo aplicativo.

## Markdown e imagens

O editor mantém o Markdown original no armazenamento. A formatação modifica a
apresentação, sem converter o arquivo inteiro a outro formato. Propriedades YAML
permanecem visíveis. Títulos, estilos inline, links, listas, citações, código,
tarefas, tabelas e imagens locais têm apresentação formatada; o trecho em edição
mostra as marcações correspondentes.

Imagens inseridas ficam inicialmente na biblioteca do app. Ao salvar, são
copiadas para uma pasta `nome-do-documento.images` ao lado do Markdown, com
referências relativas. Referências remotas permanecem no texto; o app funciona
localmente e não busca essas imagens na internet.

Ao mover ou salvar uma cópia em outra pasta, o app preserva imagens relativas
dentro da pasta do documento. Se a referência depender de um caminho externo
que não possa ser preservado com segurança, a operação é interrompida com uma
explicação; o arquivo original é preservado. Nesse caso, mova a pasta inteira
pelo Finder ou mantenha o documento no mesmo lugar.

## Onde ficam os dados

Biblioteca: `~/Library/Application Support/text-md/`.
O arquivo `library.sqlite` guarda rascunhos, histórico e sessão; `assets/` guarda
imagens internas. Para um backup completo, copie essa pasta com o app fechado.
O app não oferece sincronização nem colaboração nesta versão.

## Desenvolvimento

SwiftUI para a janela, AppKit para o editor e painéis, SQLite para persistência e
CryptoKit para detectar alterações reais no original. Não há dependências
remotas. Mínimo macOS 14; a versão foi executada neste Mac com macOS 26.7.1.

`scripts/build.py` compila e gera `build/text-md.app`; `scripts/test.py` executa os
testes com Swift Testing. Os scripts lidam com o ambiente de Command Line Tools
deste Mac, que não tem o Xcode completo. Os detalhes da verificação estão em
[VALIDACAO.md](VALIDACAO.md); o combinado está em [BRIEF.md](BRIEF.md).


## Licença

MIT — Copyright (c) 2026 Cassiano Diniz. É permitido usar, modificar e distribuir
o projeto, inclusive comercialmente, preservando o aviso de copyright e a licença.
Veja [LICENSE](LICENSE) e [os avisos de terceiros](THIRD_PARTY_NOTICES.md).


Para empacotar uma compilação existente, `scripts/package_release.py` gera
DMG, ZIP e somas SHA-256 em `build/releases/<versão>/`, sem iniciar o aplicativo.
Os scripts `ui_*` usam o primeiro plano do Mac: execute-os somente quando o
usuário autorizar disponibilizar a tela. A criação da release não executa esses scripts.
