param([Parameter(ValueFromRemainingArguments=$true)][string[]]$Documentos)
$ErrorActionPreference='Stop'
& (Join-Path $PSScriptRoot 'private-data.ps1') -PortableDirectory $PSScriptRoot
$previousDataDirectory=$env:WORDNEO_DATA_DIR
try {
    $env:WORDNEO_DATA_DIR=Join-Path $PSScriptRoot 'data'
    & (Join-Path $PSScriptRoot 'WordNeo.exe') @Documentos
} finally {
    $env:WORDNEO_DATA_DIR=$previousDataDirectory
}
