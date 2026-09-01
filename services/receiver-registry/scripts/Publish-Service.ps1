[CmdletBinding()]
param(
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Release",
    [string]$OutputPath = ""
)

$ErrorActionPreference = "Stop"
$scriptDirectory = [System.IO.Path]::GetDirectoryName($MyInvocation.MyCommand.Path)
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $scriptDirectory "..\publish\win-x64"
}
$projectPath = Join-Path $scriptDirectory "..\QAReceiverRegistry.csproj"
$nugetConfig = Join-Path $scriptDirectory "..\NuGet.Config"
$resolvedOutput = [System.IO.Path]::GetFullPath($OutputPath)
$publishRoot = [System.IO.Path]::GetFullPath((Join-Path $scriptDirectory "..\publish"))
$projectXml = [xml](Get-Content -LiteralPath $projectPath -Raw)
$versionNode = $projectXml.SelectSingleNode("/Project/PropertyGroup/Version")
if ($null -eq $versionNode -or [string]::IsNullOrWhiteSpace($versionNode.InnerText)) {
    throw "Service version was not found in $projectPath"
}
$serviceVersion = $versionNode.InnerText.Trim()

if (-not $resolvedOutput.StartsWith($publishRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "OutputPath must be inside $publishRoot"
}

if (Test-Path -LiteralPath $resolvedOutput) {
    Remove-Item -LiteralPath $resolvedOutput -Recurse -Force
}

dotnet publish $projectPath `
    --configuration $Configuration `
    --runtime win-x64 `
    --self-contained true `
    --configfile $nugetConfig `
    --output $resolvedOutput

if ($LASTEXITCODE -ne 0) {
    throw "Service publishing failed with exit code $LASTEXITCODE."
}

@("NuGet.Config", "web.config", "QAReceiverRegistry.staticwebassets.endpoints.json") |
    ForEach-Object {
        $path = Join-Path $resolvedOutput $_
        if (Test-Path -LiteralPath $path) {
            Remove-Item -LiteralPath $path -Force
        }
    }

@("Install-Service.ps1", "Update-Service.ps1", "Uninstall-Service.ps1", "Test-Service.ps1", "Import-Receivers.ps1") |
    ForEach-Object {
        Copy-Item -LiteralPath (Join-Path $scriptDirectory $_) -Destination $resolvedOutput -Force
    }
Copy-Item -LiteralPath (Join-Path $scriptDirectory "..\README.md") -Destination (Join-Path $resolvedOutput "README.md") -Force

$hashPath = Join-Path $resolvedOutput "SHA256SUMS.txt"
$resolvedOutputPrefix = $resolvedOutput.TrimEnd("\") + "\"
Get-ChildItem -LiteralPath $resolvedOutput -File -Recurse |
    Where-Object FullName -ne $hashPath |
    Sort-Object FullName |
    ForEach-Object {
        $relativePath = $_.FullName.Substring($resolvedOutputPrefix.Length)
        $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $relativePath"
    } | Set-Content -LiteralPath $hashPath -Encoding ascii

Write-Host "Published service: $resolvedOutput"
Write-Host "Integrity manifest: $hashPath"

Get-ChildItem -LiteralPath $publishRoot -Filter "QAReceiverRegistry-*-win-x64.zip*" -File |
    Remove-Item -Force
$archivePath = Join-Path $publishRoot "QAReceiverRegistry-$serviceVersion-win-x64.zip"
Compress-Archive -Path (Join-Path $resolvedOutput "*") -DestinationPath $archivePath -CompressionLevel Optimal
$archiveHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
$archiveHashPath = "$archivePath.sha256"
"$archiveHash  $([System.IO.Path]::GetFileName($archivePath))" |
    Set-Content -LiteralPath $archiveHashPath -Encoding ascii
Write-Host "Deployment archive: $archivePath"
Write-Host "Archive checksum: $archiveHashPath"
