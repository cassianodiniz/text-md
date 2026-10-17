# WordNeo

Abra `WordNeo.exe` para usar o editor. O Microsoft Word não é necessário para executar o aplicativo.

Para manter as configurações junto ao pacote, execute `Iniciar-WordNeo.ps1` no PowerShell. O script prepara somente a subpasta `data` deste pacote. Não instala componentes, altera associações de arquivos ou configura o sistema.

`WordNeo-cli.exe` oferece conversão e automação. Exemplo:

```powershell
.\WordNeo-cli.exe convert .\entrada.docx .\saida.pdf
```

Esta é uma candidata de desenvolvimento. Os resultados e limites de validação ficam no diário do projeto. Não foi demonstrada equivalência com todos os recursos do Word. Fontes disponíveis na máquina influenciam a diagramação.

O pacote preserva os créditos e as licenças do WordCraft. Documentos de teste e fontes proprietárias não são distribuídos.
