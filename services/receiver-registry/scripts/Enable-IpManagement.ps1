[CmdletBinding()]
param(
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

function Set-JsonProperty {
    param(
        [Parameter(Mandatory = $true)]$Object,
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)]$Value
    )

    if ($Object.PSObject.Properties.Name -contains $Name) {
        $Object.$Name = $Value
    }
    else {
        $Object | Add-Member -NotePropertyName $Name -NotePropertyValue $Value
    }
}

function Wait-ForHealthyService {
    param([string]$HealthUri)

    $health = $null
    $deadline = (Get-Date).AddSeconds(30)
    do {
        Start-Sleep -Seconds 1
        $service = Get-Service -Name $serviceName
        if ($service.Status -eq "Running") {
            try {
                $health = Invoke-RestMethod -Uri $HealthUri -Method Get -TimeoutSec 3
            }
            catch {
                $health = $null
            }
        }
    } until (($health -and $health.status -eq "healthy") -or (Get-Date) -ge $deadline)

    if (-not $health -or $health.status -ne "healthy") {
        throw "QA Receiver Registry did not become healthy within 30 seconds."
    }
    return $health
}

Assert-Administrator
$service = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
if (-not $service) {
    throw "Service $serviceName is not installed."
}

$configPath = Join-Path $RootPath "config\appsettings.json"
if (-not (Test-Path -LiteralPath $configPath)) {
    throw "Service configuration was not found: $configPath"
}

$configuration = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
$scheme = if ($configuration.Server.UseHttps) { "https" } else { "http" }
$healthHost = if ($configuration.Server.UseHttps) { $PublicHostName } else { $configuration.Server.ListenAddress }
$baseUri = "$scheme`://$healthHost`:$($configuration.Server.Port)"
$health = Invoke-RestMethod -Uri "$baseUri/health" -Method Get -TimeoutSec 10
if ($health.status -ne "healthy" -or $health.database.integrityStatus -ne "ok") {
    throw "The service is not healthy. IP Management was not enabled."
}
if ([int]$health.database.schemaVersion -lt 5) {
    throw "Database schema 5 is required. Run Update-Service.ps1 before enabling IP Management."
}
if ([int]$health.database.ipAddressCount -ne 254) {
    throw "The database contains $($health.database.ipAddressCount) IP addresses instead of 254."
}

$databaseBackup = Invoke-RestMethod `
    -Uri "$baseUri/api/v1/maintenance/backup" `
    -Method Post `
    -TimeoutSec 30
Write-Host "Verified database backup created: $($databaseBackup.fullPath)"

$configBackupPath = "$configPath.$(Get-Date -Format 'yyyyMMdd-HHmmss').backup"
Copy-Item -LiteralPath $configPath -Destination $configBackupPath -Force
Write-Host "Configuration backup created: $configBackupPath"

if (-not ($configuration.PSObject.Properties.Name -contains "Features") -or $null -eq $configuration.Features) {
    $configuration | Add-Member -NotePropertyName Features -NotePropertyValue ([pscustomobject]@{})
}

Set-JsonProperty -Object $configuration.Features -Name "IpManagementEnabled" -Value $true
Set-JsonProperty -Object $configuration.Features -Name "GuestPortalEnabled" -Value $false
if ($configuration.Features.PSObject.Properties.Name -contains "IpAdminKey") {
    $configuration.Features.PSObject.Properties.Remove("IpAdminKey")
}

$temporaryConfigPath = "$configPath.tmp"
$json = $configuration | ConvertTo-Json -Depth 12
[IO.File]::WriteAllText($temporaryConfigPath, $json, (New-Object Text.UTF8Encoding($false)))

try {
    Move-Item -LiteralPath $temporaryConfigPath -Destination $configPath -Force
    Restart-Service -Name $serviceName -Force
    $health = Wait-ForHealthyService -HealthUri "$baseUri/health"

    $inventory = Invoke-RestMethod `
        -Uri "$baseUri/api/v1/ip-addresses" `
        -Method Get `
        -Headers @{ "X-QA-User" = "$env:USERDOMAIN\$env:USERNAME" }
    if (@($inventory).Count -ne 254) {
        throw "The API returned $(@($inventory).Count) IP addresses instead of 254."
    }
}
catch {
    Copy-Item -LiteralPath $configBackupPath -Destination $configPath -Force
    Restart-Service -Name $serviceName -Force
    Wait-ForHealthyService -HealthUri "$baseUri/health" | Out-Null
    throw "IP Management activation failed. The previous configuration was restored. $($_.Exception.Message)"
}
finally {
    if (Test-Path -LiteralPath $temporaryConfigPath) {
        Remove-Item -LiteralPath $temporaryConfigPath -Force
    }
}

Write-Host "IP Management is enabled and its API is working."
Write-Host "Guest portal remains disabled."
Write-Host "Schema version: $($health.database.schemaVersion)"
Write-Host "IP addresses: $(@($inventory).Count)"
