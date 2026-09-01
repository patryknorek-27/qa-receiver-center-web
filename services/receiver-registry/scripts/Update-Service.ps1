[CmdletBinding()]
param(
    [string]$PublishPath = "",
    [string]$RootPath = "C:\TSQATools\QAReceiverRegistry",
    [string]$PublicHostName = "HHKNQA1-DE-DE04.eu.trimblecorp.net"
)

$ErrorActionPreference = "Stop"
$serviceName = "QAReceiverRegistry"

function Assert-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw "Run this script from an elevated PowerShell window."
    }
}

function Assert-PackageIntegrity {
    param([string]$Path)

    $manifestPath = Join-Path $Path "SHA256SUMS.txt"
    if (-not (Test-Path -LiteralPath $manifestPath)) {
        throw "Package integrity manifest was not found: $manifestPath"
    }

    foreach ($line in Get-Content -LiteralPath $manifestPath) {
        if ($line -notmatch "^([0-9a-fA-F]{64})  (.+)$") {
            throw "Package integrity manifest contains an invalid entry: $line"
        }

        $expectedHash = $Matches[1].ToLowerInvariant()
        $relativePath = $Matches[2]
        $filePath = Join-Path $Path $relativePath
        if (-not (Test-Path -LiteralPath $filePath)) {
            throw "Package file was not found: $relativePath"
        }

        $actualHash = (Get-FileHash -LiteralPath $filePath -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actualHash -ne $expectedHash) {
            throw "Package integrity check failed for $relativePath"
        }
    }
}

Assert-Administrator
$scriptDirectory = [System.IO.Path]::GetDirectoryName($MyInvocation.MyCommand.Path)
if ([string]::IsNullOrWhiteSpace($PublishPath)) {
    $PublishPath = $scriptDirectory
}
$service = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
if (-not $service) {
    throw "Service $serviceName is not installed. Use Install-Service.ps1 for the first installation."
}

$resolvedPublishPath = [System.IO.Path]::GetFullPath($PublishPath)
$sourceExecutable = Join-Path $resolvedPublishPath "QAReceiverRegistry.exe"
if (-not (Test-Path -LiteralPath $sourceExecutable)) {
    throw "Published service executable was not found: $sourceExecutable"
}
Assert-PackageIntegrity -Path $resolvedPublishPath

$appPath = Join-Path $RootPath "app"
$configPath = Join-Path $RootPath "config\appsettings.json"
if (-not (Test-Path -LiteralPath $configPath)) {
    throw "Installed service configuration was not found: $configPath"
}

$configuration = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
$scheme = if ($configuration.Server.UseHttps) { "https" } else { "http" }
$healthHost = if ($configuration.Server.UseHttps) { $PublicHostName } else { $configuration.Server.ListenAddress }
$baseUri = "$scheme`://$healthHost`:$($configuration.Server.Port)"

$health = Invoke-RestMethod -Uri "$baseUri/health" -TimeoutSec 10
if ($health.status -ne "healthy") {
    throw "The installed service is not healthy. Update was cancelled."
}
$backup = Invoke-RestMethod -Method Post -Uri "$baseUri/api/v1/maintenance/backup" -TimeoutSec 30
Write-Host "Verified database backup created: $($backup.fullPath)"

$rollbackRoot = Join-Path $RootPath "rollback"
$rollbackPath = Join-Path $rollbackRoot (Get-Date -Format "yyyyMMdd-HHmmss")
New-Item -ItemType Directory -Path $rollbackPath -Force | Out-Null
Copy-Item -Path (Join-Path $appPath "*") -Destination $rollbackPath -Recurse -Force

Stop-Service -Name $serviceName -Force
$deadline = (Get-Date).AddSeconds(30)
do {
    Start-Sleep -Milliseconds 250
    $service = Get-Service -Name $serviceName
} until ($service.Status -eq "Stopped" -or (Get-Date) -ge $deadline)
if ($service.Status -ne "Stopped") {
    throw "The service did not stop within 30 seconds."
}

try {
    Copy-Item -Path (Join-Path $resolvedPublishPath "*") -Destination $appPath -Recurse -Force
    $executablePath = Join-Path $appPath "QAReceiverRegistry.exe"
    & $executablePath --config $configPath --migrate-only
    if ($LASTEXITCODE -ne 0) {
        throw "Database migration failed with exit code $LASTEXITCODE."
    }

    $health = $null
    Start-Service -Name $serviceName
    $deadline = (Get-Date).AddSeconds(30)
    do {
        Start-Sleep -Seconds 1
        $service = Get-Service -Name $serviceName
        if ($service.Status -eq "Running") {
            try {
                $health = Invoke-RestMethod -Uri "$baseUri/health" -TimeoutSec 3
            }
            catch {
                $health = $null
            }
        }
    } until (($health -and $health.status -eq "healthy") -or (Get-Date) -ge $deadline)

    if (-not $health -or $health.status -ne "healthy") {
        throw "The updated service did not become healthy within 30 seconds."
    }
}
catch {
    Stop-Service -Name $serviceName -Force -ErrorAction SilentlyContinue
    Copy-Item -Path (Join-Path $rollbackPath "*") -Destination $appPath -Recurse -Force
    Start-Service -Name $serviceName
    throw
}

Write-Host "QA Receiver Registry update completed successfully."
Write-Host "Schema version: $($health.database.schemaVersion)"
Write-Host "Rollback files: $rollbackPath"
