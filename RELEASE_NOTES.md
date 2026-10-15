# text-md 0.3.2

Correções dos 18 problemas da primeira auditoria e do ciclo de fechamento,
biblioteca e restauração de janelas. Licença MIT.

- **⌘W fecha só a aba:** o rascunho permanece na biblioteca e na lateral.
- **Dock sem janelas:** abre a Biblioteca, com **Restaurar última janela**.
- Recuperação de janelas mantém abas, ordem, seleção, navegação e largura da lateral.
- Histórico de fechamento persiste após encerrar; janelas em restauração reservam seus documentos para evitar duplicação.
- Cópias antigas divergentes ficam acessíveis sem associação de escrita ao original, mantendo a base de imagens e links.
- Falhar ao restaurar uma versão mantém o backup pendente para a próxima tentativa de gravação.

- Lixeira com **Voltar ao editor** e Escape, preservando o histórico de desfazer/refazer.
- Largura da lateral guardada por janela. Um único botão **+** para criar rascunhos.
- Desfazer/refazer respeitam o campo focado; comandos de documento ficam indisponíveis na Lixeira.
- Fechamento de aba/janela só oculta documentos após guardar o estado com sucesso.
- Busca recebe o foco ao abrir a lateral, mostra caminho e trecho e cancela trabalhos substituídos.
- Markdown interpretado com Swift Markdown/CommonMark/GFM: código literal, escapes, links por referência, títulos, tabelas e âncoras locais.
- Salvar preserva exemplos literais de referências internas. Mover/copiar conserva imagens reconhecidas, independentemente da extensão, ou informa o impedimento.
- Movimentar atalhos simbólicos é recusado para evitar referências quebradas. Salvar sobre outro arquivo com rascunho guardado é bloqueado para preservar ambos.
- Apagar tem o mesmo efeito nos dois contextos. Restaurar na lixeira interna recupera um rascunho separado, sem recriar o arquivo original.
- Limpeza remove mídias internas órfãs antigas e conserva as usadas por rascunhos ou versões.
- **Renomear** disponível no menu e como ação acessível, além do duplo clique.

Detalhamento e limites: [AUDIT_FIXES.md](AUDIT_FIXES.md) e [VALIDACAO.md](VALIDACAO.md).

## Compatibilidade

Mac Apple Silicon (M1 ou posterior), macOS 14 ou posterior. Sem versão Intel.
O aplicativo continua com assinatura local ad hoc, sem Developer ID ou notarização.
A biblioteca pessoal não é substituída pela atualização do aplicativo.

## Baixar e instalar

Baixe `text-md-0.3.2-macos-arm64.dmg`, abra e arraste **text-md.app** para
**Applications / Aplicativos**. Encerre normalmente a versão anterior antes de
substituí-la. Seus rascunhos ficam preservados na biblioteca do usuário.

ZIP alternativo e `SHA256SUMS.txt` também acompanham a release.

A versão não tem Developer ID nem notarização da Apple. Se o macOS bloquear a
primeira abertura, tente abrir o app e use **Ajustes do Sistema → Privacidade e
Segurança → Abrir Mesmo Assim**. [Orientação oficial](https://support.apple.com/en-us/102445).

## Verificação e licença

67 testes passaram na preparação das correções. Os pacotes são conferidos sem
iniciar o aplicativo. Os gestos da interface não foram exercitados nesta revisão.

MIT — Copyright (c) 2026 Cassiano Diniz. Os avisos integrais de Swift Markdown,
swift-cmark e tokenização acompanham o aplicativo e a imagem de instalação.
