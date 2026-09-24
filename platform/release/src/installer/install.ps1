$ErrorActionPreference = 'Stop'

# select the Windows distribution
$repository = '__REPOSITORY__'
$catalog = Invoke-RestMethod ($repository + 'downloads.json')
$release = $catalog.downloads.'x86_64-pc-windows-msvc'.installers.exe
if (-not $release) { throw 'A release is not available for this platform.' }
if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne 'X64') {
    throw 'This installer requires Windows x64.'
}
if ($release.url -notmatch ('^' + [regex]::Escape($repository) + 'targets/[a-f0-9]{64}\.x86_64-pc-windows-msvc\.exe$')) {
    throw 'Invalid distribution URL.'
}

# verify the same native installer offered by the website
$temporary = Join-Path ([System.IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $temporary | Out-Null
try {
    $executable = Join-Path $temporary 'Destack.exe'
    Invoke-WebRequest $release.url -OutFile $executable
    $digest = (Get-FileHash $executable -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($digest -ne $release.sha256) { throw 'Download digest does not match.' }
    if ((Get-Item -LiteralPath $executable).Length -ne $release.size) { throw 'Download size does not match.' }
    $signature = Get-AuthenticodeSignature -LiteralPath $executable
    if ($signature.Status -ne 'Valid') { throw 'invalid installer executable signature' }
    if ($signature.SignerCertificate.GetNameInfo([System.Security.Cryptography.X509Certificates.X509NameType]::SimpleName, $false) -ne 'Symbol Industries GmbH') {
        throw 'unexpected installer publisher'
    }
    $installation = Start-Process -FilePath $executable -ArgumentList '/S' -Wait -PassThru
    if ($installation.ExitCode -ne 0) { throw 'Installation failed.' }
} finally {
    Remove-Item -Recurse -Force -LiteralPath $temporary
}
