Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# release source
$DestackRepository = if ($env:DESTACK_REPOSITORY) { $env:DESTACK_REPOSITORY } else { "destack-sh/destack" }
$DestackReleaseBaseUrl = if ($env:DESTACK_RELEASE_BASE_URL) { $env:DESTACK_RELEASE_BASE_URL } else { "https://github.com/$DestackRepository/releases/download" }
$DestackApiBaseUrl = if ($env:DESTACK_API_BASE_URL) { $env:DESTACK_API_BASE_URL } else { "https://api.github.com/repos/$DestackRepository" }
$DestackGithubToken = if ($env:DESTACK_GITHUB_TOKEN) { $env:DESTACK_GITHUB_TOKEN } else { "" }
$DestackUserAgent = if ($env:DESTACK_CURL_USER_AGENT) { $env:DESTACK_CURL_USER_AGENT } else { "destack-cli-installer" }

# install configuration
$DestackVersionInput = if ($env:DESTACK_VERSION) { $env:DESTACK_VERSION } else { "latest" }
$DestackInstallDir = if ($env:DESTACK_INSTALL) { $env:DESTACK_INSTALL } else { Join-Path $HOME ".destack\bin" }
$DestackNoModifyPath = if ($env:DESTACK_NO_MODIFY_PATH) { $env:DESTACK_NO_MODIFY_PATH } else { "0" }

# print an informational message
function Write-InstallInfo {
    param([string]$Message)
    Write-Host "info: $Message"
}

# throw an installer error
function Throw-InstallError {
    param([string]$Message)
    throw "error: $Message"
}

# resolve request headers for github api and release downloads
function Resolve-RequestHeaders {
    $headers = @{
        "User-Agent" = $DestackUserAgent
    }

    # include github auth when a token is configured
    if (-not [string]::IsNullOrWhiteSpace($DestackGithubToken)) {
        $headers["Authorization"] = "Bearer $DestackGithubToken"
    }

    return $headers
}

# resolve the supported windows architecture
function Resolve-Architecture {
    $architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture

    # support x64 releases
    if ($architecture -eq [System.Runtime.InteropServices.Architecture]::X64) {
        return "x64"
    }

    # support arm64 releases when published
    if ($architecture -eq [System.Runtime.InteropServices.Architecture]::Arm64) {
        return "arm64"
    }

    Throw-InstallError "unsupported architecture: $architecture"
}

# resolve the rust target triple
function Resolve-TargetTriple {
    param([string]$Architecture)

    # map x64 to the current release target
    if ($Architecture -eq "x64") {
        return "x86_64-pc-windows-msvc"
    }

    # arm64 targets are not published yet
    if ($Architecture -eq "arm64") {
        Throw-InstallError "arm64 windows targets are not published yet, use npm or build from source"
    }

    Throw-InstallError "unsupported architecture: $Architecture"
}

# resolve a release version and tag
function Resolve-ReleaseVersion {
    param([string]$VersionInput, [string]$ApiBaseUrl)

    # use the latest release tag from github
    if ($VersionInput -eq "latest") {
        $headers = Resolve-RequestHeaders
        $release = Invoke-RestMethod -Uri "$ApiBaseUrl/releases/latest" -Headers $headers
        if (-not $release.tag_name) {
            Throw-InstallError "failed to resolve latest release tag"
        }

        $tag = [string]$release.tag_name
        $version = $tag.TrimStart("v")
        return @{
            version = $version
            tag = $tag
        }
    }

    # preserve explicit v-prefixed tags
    if ($VersionInput.StartsWith("v")) {
        return @{
            version = $VersionInput.TrimStart("v")
            tag = $VersionInput
        }
    }

    # normalize raw semantic versions to v tags
    return @{
        version = $VersionInput
        tag = "v$VersionInput"
    }
}

# download a file from url
function Download-File {
    param([string]$Url, [string]$DestinationPath)

    $headers = Resolve-RequestHeaders
    Invoke-WebRequest -Uri $Url -Headers $headers -OutFile $DestinationPath
}

# resolve expected sha256 hash from checksum file
function Resolve-ExpectedChecksum {
    param([string]$ChecksumsPath, [string]$ArchiveName)

    $escapedArchiveName = [regex]::Escape($ArchiveName)
    $lines = Get-Content -Path $ChecksumsPath
    $line = $lines | Where-Object { $_ -match "(\*| )$escapedArchiveName$" } | Select-Object -First 1
    if (-not $line) {
        Throw-InstallError "checksum entry not found for $ArchiveName"
    }

    $parts = $line -split "\s+"
    return $parts[0].ToLowerInvariant()
}

# verify archive checksum with sha256
function Verify-Checksum {
    param([string]$ArchivePath, [string]$ChecksumsPath, [string]$ArchiveName)

    $expectedHash = Resolve-ExpectedChecksum -ChecksumsPath $ChecksumsPath -ArchiveName $ArchiveName
    $actualHash = (Get-FileHash -Path $ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()

    if ($expectedHash -ne $actualHash) {
        Throw-InstallError "checksum mismatch for $ArchiveName"
    }
}

# resolve a binary path from extracted files
function Resolve-BinaryPath {
    param([string]$ExtractDirectory, [string]$BinaryName)

    $file = Get-ChildItem -Path $ExtractDirectory -Recurse -File -Filter $BinaryName | Select-Object -First 1
    if (-not $file) {
        Throw-InstallError "missing binary in archive: $BinaryName"
    }

    return $file.FullName
}

# install binaries to the destination directory
function Install-Binaries {
    param([string]$ExtractDirectory, [string]$InstallDirectory)

    New-Item -ItemType Directory -Path $InstallDirectory -Force | Out-Null
    $binaryNames = @("destack.exe", "ds.exe", "dsc.exe")

    # copy each required executable
    foreach ($binaryName in $binaryNames) {
        $sourcePath = Resolve-BinaryPath -ExtractDirectory $ExtractDirectory -BinaryName $binaryName
        $destinationPath = Join-Path $InstallDirectory $binaryName
        Copy-Item -Path $sourcePath -Destination $destinationPath -Force
    }
}

# update user path unless disabled
function Ensure-PathEntry {
    param([string]$InstallDirectory, [string]$NoModifyPath)

    if ($NoModifyPath -eq "1") {
        Write-InstallInfo "skipping path update because DESTACK_NO_MODIFY_PATH=1"
        return
    }

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($null -eq $userPath) {
        $userPath = ""
    }

    if ($userPath.Split(";") -contains $InstallDirectory) {
        Write-InstallInfo "install directory already present in user PATH"
        return
    }

    if ([string]::IsNullOrWhiteSpace($userPath)) {
        $updatedPath = $InstallDirectory
    } else {
        $updatedPath = "$userPath;$InstallDirectory"
    }

    [Environment]::SetEnvironmentVariable("Path", $updatedPath, "User")
    Write-InstallInfo "added $InstallDirectory to user PATH, restart your shell"
}

# run the installer flow
function Invoke-Install {
    # resolve runtime and release metadata
    $architecture = Resolve-Architecture
    $targetTriple = Resolve-TargetTriple -Architecture $architecture
    $release = Resolve-ReleaseVersion -VersionInput $DestackVersionInput -ApiBaseUrl $DestackApiBaseUrl
    $version = [string]$release.version
    $tag = [string]$release.tag

    Write-InstallInfo "installing destack $version for $targetTriple"

    # resolve release asset names
    $archiveName = "destack-$version-$targetTriple.zip"
    $archiveUrl = "$DestackReleaseBaseUrl/$tag/$archiveName"
    $checksumsName = "SHA256SUMS"
    $checksumsUrl = "$DestackReleaseBaseUrl/$tag/$checksumsName"

    # create a temporary workspace
    $tempDirectory = Join-Path ([System.IO.Path]::GetTempPath()) ("destack-install-" + [Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $tempDirectory -Force | Out-Null

    try {
        $archivePath = Join-Path $tempDirectory $archiveName
        $checksumsPath = Join-Path $tempDirectory $checksumsName
        $extractDirectory = Join-Path $tempDirectory "extract"

        # download release files
        Download-File -Url $archiveUrl -DestinationPath $archivePath
        Download-File -Url $checksumsUrl -DestinationPath $checksumsPath

        # verify and extract archive
        Verify-Checksum -ArchivePath $archivePath -ChecksumsPath $checksumsPath -ArchiveName $archiveName
        New-Item -ItemType Directory -Path $extractDirectory -Force | Out-Null
        Expand-Archive -Path $archivePath -DestinationPath $extractDirectory -Force

        # install binaries and update path
        Install-Binaries -ExtractDirectory $extractDirectory -InstallDirectory $DestackInstallDir
        Ensure-PathEntry -InstallDirectory $DestackInstallDir -NoModifyPath $DestackNoModifyPath

        # print final guidance
        Write-InstallInfo "installed binaries into $DestackInstallDir"
        Write-InstallInfo "run: destack --version"
    } finally {
        Remove-Item -Path $tempDirectory -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Invoke-Install
