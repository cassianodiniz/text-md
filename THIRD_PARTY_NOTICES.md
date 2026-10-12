# Componentes de terceiros

O código do text-md é distribuído sob a [licença MIT](LICENSE).

## Vocabulários e padrões de tokenização

Os arquivos `cl100k_base.tiktoken`, `o200k_base.tiktoken` e `patterns.json` em
`Sources/TextMDCore/Resources` são derivados do tiktoken 0.14.0 da OpenAI,
distribuído sob licença MIT.

Copyright (c) 2022 OpenAI, Shantanu Jain.

O texto integral do aviso está em
[OPENAI-TOKENIZER-LICENSE.txt](Sources/TextMDCore/Resources/OPENAI-TOKENIZER-LICENSE.txt)
e acompanha os recursos incorporados no aplicativo.

Fonte: https://github.com/openai/tiktoken

## Componentes do sistema

AppKit, SwiftUI, Foundation, CryptoKit e SQLite são usados por meio das
bibliotecas do macOS. O pacote não redistribui esses frameworks do sistema.
Não são incluídas fontes, ícones ou folhas de estilo do Typora.
