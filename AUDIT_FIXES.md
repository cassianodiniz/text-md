# Correções da auditoria — text-md 0.3.2

14/10/2026, horário de Brasília. Os IDs correspondem aos 18 achados da auditoria.
A ordem abaixo segue a prioridade recomendada: integridade dos dados, operações
com arquivos, navegação, Markdown/busca, consistência/acessibilidade e limpeza.

| Ordem | ID | Correção | Evidência desta revisão |
| --- | --- | --- | --- |
| 1 | A04 | Menu nativo de desfazer/refazer segue o campo focado; editor e histórico pertencem ao documento. A mesma instância de texto sobrevive à troca de tela. | Revisão do encaminhamento nativo; teste de desfazer/refazer ao passar pela Lixeira, sem janela. |
| 2 | A05 | Contexto de documento exigido para formatação, salvar, diff, histórico, busca no texto e imagens; contadores e status acompanham a tela. | Teste de comandos na Lixeira sem alterar o texto oculto; revisão de menus e barra. |
| 3 | A11 | Exportação reescreve somente imagens reconhecidas pelo parser, nunca exemplos literais do prefixo interno. | Salvar exemplos existentes/inexistentes e imagens inline/por referência em pasta temporária. |
| 4 | A12 | Mesma interpretação de imagem na apresentação e nas operações; título, rótulo, fragmento e formatos como BMP preservados. Falha segura em dependências externas/colisões. | Testes de mover/copiar, bytes e dependências; destino com parênteses também coberto. |
| 5 | A13 | Movimento de link simbólico é recusado com orientação para usar o arquivo real ou Finder. | Teste com link relativo e arquivo real intactos. |
| 6 | A17 | Antes de salvar/mover, destino com outro rascunho guardado é rejeitado; não cria duas identidades para o mesmo arquivo. | Teste com dois originais e duas edições; ambos preservados. |
| 7 | A08 | Fechamento grava sessão e ocultação numa transação; recusa mantém abas e visibilidade, inclusive na última aba. | Falha de gravação e falha apenas da sessão injetadas em biblioteca descartável. |
| 8 | A01 | **Voltar ao editor** e Escape saem da Lixeira; seleção e editor anterior mantidos. | Teste de transição e retenção do editor; ação visível revisada no código. |
| 9 | A02 | Largura faz parte do estado de cada janela, dentro de 190–350 pontos; formato antigo continua legível. | Testes de serialização/restauração e revisão da medição do divisor. Arraste real não executado. |
| 10 | A03 | Botão inferior de novo rascunho removido; permanecem + e Command+T. Um único item de aumentar zoom, aceitando + e =. | Revisão das declarações de botão, menu e atalho. |
| 11 | A10 | Interpretação compartilhada com Swift Markdown para código, escapes, links, imagens, títulos Setext, tabelas e projeção de contagem; links locais encaminham âncoras. | Casos de Markdown e estilo executados sem janela, com Unicode, CRLF, títulos de links e definições falsas. Gesto real de link/rolagem não executado. |
| 12 | A06 | Pedido de foco persiste até o campo de busca existir; abrir busca revela a lateral. | Teste do estado e revisão da tarefa de foco. Digitação real não executada. |
| 13 | A07 | Resultados mostram caminho e trecho, com descrição acessível que distingue nomes iguais. | Teste dos dados de busca e revisão das linhas. |
| 14 | A14 | Tela distingue rascunho interno e Lixeira do Mac; **Restaurar rascunho** sempre recupera uma cópia separada do arquivo. | Teste com outro arquivo ocupando o caminho antigo: texto recuperado e arquivo substituto permanecem intactos. |
| 15 | A15 | **Apagar** na recuperação de fechados usa a mesma operação da lateral, com confirmação antes de mandar arquivo real à Lixeira do Mac. | Revisão dos dois encaminhamentos; nenhum arquivo pessoal foi apagado para validar. |
| 16 | A18 | Menu **Renomear** e ação acessível iniciam o mesmo campo de edição do duplo clique. | Revisão da entrada de menu e acessibilidade; VoiceOver/teclado real não executados. |
| 17 | A09 | Cancelamento chega ao trabalho de busca; varredura consulta o sinal entre arquivos e leituras. | Teste de cancelamento e revisão do ciclo de vida. Não foi medido desempenho em pastas pessoais. |
| 18 | A16 | Após expirar registros, coleta apenas imagens internas sem referência e antigas há mais de 90 dias; considera rascunhos ativos, fechados, lixeira e histórico. | Testes de mídia compartilhada, referências codificadas, versões, órfãs e arquivos recentes. |

## Preservação e limites

51 testes passaram: 43 de dados/Markdown e 8 do modelo/editor. Os testes usam
arquivos e bibliotecas descartáveis. Componentes AppKit foram criados sem janela;
nenhum aplicativo, simulador, captura ou script de interface foi aberto.

As ações de mover, apagar, renomear e salvar continuam disponíveis ao usuário;
a atualização em si não as executa sobre seus documentos. O formato SQLite da
biblioteca não foi substituído. Há backup consistente da biblioteca e do app
anterior antes da instalação. A biblioteca do usuário não é usada nos testes.

Os pontos de interface acima foram corrigidos no código; não se afirma validação
visual ou execução de todos os gestos sem disponibilizar a tela. A apresentação
é inspirada no Whitey, sem promessa de igualdade de pixels ou suporte a todas
as extensões do Typora. Tokens continuam incluindo todo o Markdown.

## Referências da implementação

- [Swift Markdown 0.5.0](https://github.com/swiftlang/swift-markdown/tree/0.5.0)
- [CommonMark](https://spec.commonmark.org/0.31.2/)
- [AppKit — responder chain](https://developer.apple.com/documentation/appkit/handling-key-events)

Versões dos componentes fixadas em `Package.resolved`; avisos integrais em `Licenses/`.


## Segunda auditoria — fechamento e restauração

Decisões do usuário em 15/10/2026: Command+W fecha só a aba; rascunho continua
na biblioteca/lateral. O Dock, sem janelas, mostra biblioteca com restauração
explícita da última janela. Isso substitui o contrato anterior de ocultar fechados.

- Biblioteca independente das abas, incluindo documentos ocultos por versões antigas.
- Histórico durável de abas/janelas, com grupo, ordem, seleção e navegação.
- Janela sem abas não cria rascunho vazio automaticamente; restauração pode usar
  a janela vazia da biblioteca, evitando outra janela desnecessária.
- Reservas de janelas pendentes impedem que o mesmo documento seja aberto em duas.
- Fechamento e transferência persistem antes de retirar acesso; fechamento de
  aba ativa escolhe a vizinha e fechamento de aba de fundo mantém a seleção.
- Duplicatas antigas mantêm todos os textos como cópias recuperadas e conservam
  uma base de referências independente da associação ao arquivo original.
- Retry de restauração conserva o backup anterior; atualizações externas
  sincronizam o editor em cache antes de uma gravação de posição.
- Renomear/mover recusa destino reservado por outro rascunho mesmo se seu arquivo
  original estiver ausente.
- Janelas usam títulos de documento/Biblioteca. Não foi introduzida outra camada
  de armazenamento de geometria de janela sem comprovação de falha no mecanismo
  do SwiftUI; a largura da lateral é explicitamente persistida.

A validação cobre o modelo e armazenamento com dados fictícios; nenhum clique
real no Dock ou ciclo de eventos NSWindow foi automatizado nesta revisão.
