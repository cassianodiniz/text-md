# WordNeo 012

Snapshot do código da candidata **012**, baseado no [WordCraft](https://github.com/storytold/wordcraft) v0.7.0, commit `fc41918814f62645b6746ed68e71e8c2c2e85d6f`.

Este repositório contém o workspace Rust completo, Cargo.lock, assets abertos necessários, testes sintéticos, licenças e scripts de compilação. Não contém documentos privados, PDFs de validação, caches, credenciais, dados de usuário nem correções posteriores à012. O código do aplicativo permanece igual ao snapshot012; a publicação não retoma seu desenvolvimento.

## Compilar no Windows

Pré-requisitos: Git, Rust/Cargo com suporte à edição2024 (o build verificado utilizou rustc1.99.1) e ferramentas nativas do alvo escolhido. Para `x86_64-pc-windows-msvc`, instale Visual Studio Build Tools com C++ e Windows SDK e use seu Developer PowerShell. Para `x86_64-pc-windows-gnu`, disponibilize MinGW-w64/GCC/dlltool no PATH. O compilador de recursos Windows é opcional; sua ausência gera aviso sobre ícone/versão.

```powershell
git clone https://github.com/cassianodiniz/apple-smart-apps.git
cd apple-smart-apps/apps/wordneo
.uild.ps1
```

O Cargo baixa as dependências fixadas em Cargo.lock na primeira compilação. Use `-Offline` apenas se elas já estiverem no cache. O script usa ferramentas do PATH, não instala programas e não depende do repositório Comercial. `-CargoPath` permite selecionar outro cargo.exe; `-TargetDirectory` permite selecionar um cache de build. As variáveis definidas pelo script valem apenas para o processo atual.

Saída: `dist/wordneo-012/WordNeo.exe`, CLI, licenças e launcher. Abra com `Iniciar-WordNeo.ps1` para usar a pasta de dados com permissões privadas. O pacote não inclui fontes proprietárias: Calibri e outras fontes são usadas do sistema quando instaladas.

Build direto, sem montar a pasta portátil:

```powershell
cargo build --locked --release -p wordcraft -p wordcraft-cli
```

No GNU, prefira o script: ele gera a pequena biblioteca de importação shlwapi usada pela interface. O caminho MSVC depende das ferramentas de compilação da Microsoft. Arquivos executáveis gerados ficam em target/release. Em Linux/macOS, o workspace upstream também oferece build Cargo; as dependências do sistema estão no [README upstream](README-UPSTREAM.md). A entrega012 aqui corresponde ao Windowsx64.

## Identidade e limites

`publication/candidate-012.json` registra a base e os hashes dos96 arquivos que compõem as alterações acumuladas da012. `publication/source-files.json` relaciona os hashes de todos os arquivos da árvore publicada, sem incluir metadados Git nem saídas de compilação. O número interno dos crates continua0.7.0, para preservar o código dessa candidata.

A012 incorpora reparos anteriores de DOCX/tabelas/bibliografia e corrige a posição padrão em entrelinha fixa. Persistem diferenças na Calibri simples, no desenho dos alinhamentos explícitos topo/centro/base e em recursos ainda sem equivalência comprovada com o Word. Preservar a identificação APA6 não garante formataçãoAPA6 equivalente. Nenhuma equivalência completa ou percentual de fidelidade é declarado.

A publicação verifica integridade, privacidade e compilação; não repete a suíte inteira nem abre os quatro documentos privados. Workflows de publicação do projeto original foram omitidos para não ativar automações de release neste novo repositório.

## Licenças e créditos

Código WordCraft sob MIT OU Apache-2.0; consulte LICENSE-MIT, LICENSE-APACHE, NOTICE e ATTRIBUTION.md. Assets e fontes abertos mantêm suas licenças específicas e atribuições no repositório. A documentação upstream está em README-UPSTREAM.md. As imagens da marca ArtCraft foram omitidas conforme docs/brand/LICENSE-brand.txt; os créditos textuais e demais avisos foram preservados.
