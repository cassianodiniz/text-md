param([Parameter(Mandatory=$true)][string]$PortableDirectory)
$ErrorActionPreference='Stop'
Import-Module (Join-Path $PSHOME 'Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1') -ErrorAction Stop
$resolvedPortable=(Resolve-Path -LiteralPath $PortableDirectory).Path
if(-not (Test-Path -LiteralPath (Join-Path $resolvedPortable 'WordNeo.exe') -PathType Leaf)){throw 'This is not a WordNeo portable directory'}
$dataDirectory=Join-Path $resolvedPortable 'data'
New-Item -ItemType Directory -Path $dataDirectory -Force | Out-Null
$dataItem=Get-Item -LiteralPath $dataDirectory -Force
if(($dataItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0){throw 'The WordNeo data directory must not be a link'}
$currentSid=[System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
& (Join-Path $env:SystemRoot 'System32\icacls.exe') $dataDirectory '/inheritance:r' '/grant:r' ("*"+$currentSid+':(OI)(CI)F') '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-544:(OI)(CI)F' | Out-Null
if($LASTEXITCODE -ne 0){throw 'Cannot protect the WordNeo data directory'}
$allowedSids=@($currentSid,'S-1-5-18','S-1-5-32-544')
foreach($entry in @((Get-Acl -LiteralPath $dataDirectory).Access)) {
    $entrySid=$entry.IdentityReference.Translate([System.Security.Principal.SecurityIdentifier]).Value
    if($entry.AccessControlType -eq 'Allow' -and $allowedSids -notcontains $entrySid) {
        & (Join-Path $env:SystemRoot 'System32\icacls.exe') $dataDirectory '/remove:g' ('*'+$entrySid) | Out-Null
        if($LASTEXITCODE -ne 0){throw 'Cannot remove broad access from WordNeo data'}
    }
}
$finalAcl=Get-Acl -LiteralPath $dataDirectory
if($allowedSids -notcontains $finalAcl.GetOwner([System.Security.Principal.SecurityIdentifier]).Value){throw 'WordNeo data has an untrusted owner'}
foreach($entry in $finalAcl.Access) {
    $entrySid=$entry.IdentityReference.Translate([System.Security.Principal.SecurityIdentifier]).Value
    if($entry.AccessControlType -eq 'Allow' -and $allowedSids -notcontains $entrySid){throw 'WordNeo data permissions remain too broad'}
}
