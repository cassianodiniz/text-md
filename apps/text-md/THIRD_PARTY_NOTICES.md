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

## Interpretação de Markdown

Swift Markdown 0.5.0, do projeto Swift, interpreta a sintaxe CommonMark/GFM.
É distribuído sob Apache License 2.0 com Runtime Library Exception. O texto
integral está em [Licenses/SwiftMarkdown.txt](Licenses/SwiftMarkdown.txt).

Fonte: https://github.com/swiftlang/swift-markdown/tree/0.5.0

Seu componente swift-cmark 0.9.0 inclui os avisos BSD, MIT e demais atribuições
descritos integralmente em [Licenses/SwiftCMark.txt](Licenses/SwiftCMark.txt).

Fonte: https://github.com/swiftlang/swift-cmark/tree/0.9.0

Esses textos acompanham o aplicativo em `Contents/Resources/Licenses`.
A licença MIT do código do text-md permanece a mesma.

## Componentes do sistema

AppKit, SwiftUI, Foundation, CryptoKit e SQLite são usados por meio das
bibliotecas do macOS. O pacote não redistribui esses frameworks do sistema.
Não são incluídas fontes, ícones ou folhas de estilo do Typora.
