# text-md 0.3.0

Editor Markdown nativo para macOS, agora disponível em pacote de instalação e sob licença MIT.

## Novidades

- Aparência inspirada no Whitey, com abas contínuas e lateral serifada.
- Fixados recolhíveis, pins nas pastas e navegação pela pasta atual.
- Clique simples na aba atual, Command+clique em nova aba e referências Markdown locais.
- Múltiplas janelas na mesma instância; arraste de abas entre janelas ou para fora.
- Atalhos de abas/janelas e zoom compartilhado com Command+mais, menos e zero.
- Propriedades YAML sem quebra de linhas, com rolagem horizontal no próprio bloco.
- Rascunhos persistentes sem alterar o original até salvar, histórico e diff editável.
- Contagem local de palavras, linhas, caracteres e tokens, sem envio dos seus textos.

## Download e instalação

**Mac Apple Silicon (M1 ou posterior), macOS 14 ou posterior.** Esta release não inclui uma versão para Macs Intel.

1. Baixe `text-md-0.3.0-macos-arm64.dmg`.
2. Abra a imagem e arraste **text-md.app** para **Applications / Aplicativos**.
3. Ejete a imagem e abra o aplicativo em Aplicativos.

Se estiver atualizando, encerre normalmente a versão anterior antes de substituir o app. Seus rascunhos ficam em `~/Library/Application Support/text-md/` e não são apagados pela atualização.

O arquivo ZIP contém o mesmo aplicativo. `SHA256SUMS.txt` contém as somas dos dois pacotes.

## Primeira abertura

O pacote possui assinatura local ad hoc; **não possui certificado Developer ID nem notarização da Apple**. Se o macOS bloquear por não identificar o desenvolvedor, tente abrir o app e depois use **Ajustes do Sistema → Privacidade e Segurança → Abrir Mesmo Assim**.

[Orientação oficial da Apple](https://support.apple.com/en-us/102445). Não é necessário desativar globalmente as proteções do Mac.

## Licença

**MIT — Copyright (c) 2026 Cassiano Diniz.**

[Licença do projeto](https://github.com/cassianodiniz/text-md/blob/v0.3.0/LICENSE) · [Avisos de terceiros](https://github.com/cassianodiniz/text-md/blob/v0.3.0/THIRD_PARTY_NOTICES.md). As licenças também acompanham o aplicativo.
