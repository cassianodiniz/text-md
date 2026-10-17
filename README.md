# Apple Smart Apps

Repositório dos aplicativos do Cassiano. Cada aplicativo mantém seu próprio código, dependências, instruções e licenças.

| Aplicativo | Plataforma / estado | Código e instruções |
|---|---|---|
| text-md | Editor Markdown para macOS14+, versão0.3.2 | [apps/text-md](apps/text-md) |
| WordNeo | Editor DOCX; snapshot da candidata012, compilação Windowsx64 verificada | [apps/wordneo](apps/wordneo) |

## text-md

As releases existentes foram preservadas: [text-md0.3.2](https://github.com/cassianodiniz/apple-smart-apps/releases/tag/v0.3.2). Para compilar, use um Mac com Swift/Xcode e Python3:

```sh
cd apps/text-md
python3 scripts/build.py
```

Consulte o [guia completo](apps/text-md/README.md), as [licenças](apps/text-md/THIRD_PARTY_NOTICES.md) e a [validação já registrada](apps/text-md/VALIDACAO.md).

## WordNeo012

Código congelado da versão012; esta consolidação não retoma as correções. Com Rust e as ferramentas nativas do Windows instaladas:

```powershell
cd apps/wordneo
.uild.ps1
```

Consulte [compilação e limites](apps/wordneo/README.md) e [identidade do snapshot012](apps/wordneo/publication/candidate-012.json). O build da GUI e CLI foi verificado; documentos privados, resultados de testes com esses documentos, dados de usuário e credenciais não fazem parte deste repositório.

## Organização e licenças

- Cada aplicativo é independente; execute seus comandos dentro da respectiva pasta.
- O histórico e as tags/releases do text-md foram preservados na renomeação deste repositório.
- O código da012 foi importado como snapshot, sem incorporar o histórico do repositório privado anterior.
- text-md: [MIT](apps/text-md/LICENSE). WordNeo: [MIT](apps/wordneo/LICENSE-MIT) ou [Apache-2.0](apps/wordneo/LICENSE-APACHE), com licenças específicas dos assets em [NOTICE](apps/wordneo/NOTICE) e [ATTRIBUTION.md](apps/wordneo/ATTRIBUTION.md).
- Imagens de marca ArtCraft foram omitidas conforme a licença de marca upstream. Os créditos e avisos legais foram preservados.

[Registro da consolidação](publication/consolidation.json).
