[CmdletBinding()]
param(
    [string]$PublishPath = "",
    [string]$RootPath = "C:\TSQATools\QAReceiverRegistry",
    [string]$BackupPath = "C:\TSQATools\Backups\QAReceiverRegistry",
    [string]$ListenAddress = "10.2.157.23",
    [string]$PublicHostName = "HHKNQA1-DE-DE04.eu.trimblecorp.net",
    [ValidateRange(1024, 65535)]
    [int]$Port = 8787,
    [switch]$UseHttps,
    [string]$CertificateThumbprint = ""
)

$ErrorActionPreference = "Stop"
$serviceName = "QAReceiverRegistry"
$serviceDisplayName = "QA Receiver Registry"
$firewallRuleName = "QA Receiver Registry TCP $Port"

function Assert-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw "Run this script from an elevated PowerShell window."
    }
}

function Grant-CertificatePrivateKeyRead {
    param([Security.Cryptography.X509Certificates.X509Certificate2]$Certificate)

    $privateKey = [Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPrivateKey($Certificate)
    if ($null -eq $privateKey) {
        throw "The certificate must use an RSA private key."
    }

    try {
        if ($privateKey -is [Security.Cryptography.RSACng]) {
            $keyFileName = $privateKey.Key.UniqueName
            $candidatePaths = @(
                (Join-Path $env:ProgramData "Microsoft\Crypto\Keys\$keyFileName"),
                (Join-Path $env:ProgramData "Microsoft\Crypto\RSA\MachineKeys\$keyFileName")
            )
            $keyPath = $candidatePaths |
                Where-Object { Test-Path -LiteralPath $_ } |
                Select-Object -First 1
        }
        elseif ($privateKey -is [Security.Cryptography.RSACryptoServiceProvider]) {
            $containerName = $privateKey.CspKeyContainerInfo.UniqueKeyContainerName
            $keyPath = Join-Path $env:ProgramData "Microsoft\Crypto\RSA\MachineKeys\$containerName"
        }
        else {
            throw "Unsupported certificate private-key provider: $($privateKey.GetType().FullName)"
        }
    }
    finally {
        $privateKey.Dispose()
    }

    if ([string]::IsNullOrWhiteSpace($keyPath) -or -not (Test-Path -LiteralPath $keyPath)) {
        throw "Certificate private-key file was not found for provider $($Certificate.GetKeyAlgorithm())."
    }

    & icacls.exe $keyPath /grant "*S-1-5-19:R" | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "Could not grant Local Service access to the certificate private key."
    }
}

function Grant-DirectoryAccess {
    param(
        [string]$Path,
        [ValidateSet("RX", "M")]
        [string]$Permission
    )

    & icacls.exe $Path /grant "*S-1-5-19:(OI)(CI)$Permission" | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "Could not grant Local Service $Permission access to $Path"
    }
}

Assert-Administrator
$scriptDirectory = [System.IO.Path]::GetDirectoryName($MyInvocation.MyCommand.Path)
if ([string]::IsNullOrWhiteSpace($PublishPath)) {
    $PublishPath = $scriptDirectory
}

if (Get-Service -Name $serviceName -ErrorAction SilentlyContinue) {
    throw "Service $serviceName already exists. Use the update procedure instead of reinstalling it."
}

$resolvedPublishPath = [System.IO.Path]::GetFullPath($PublishPath)
$sourceExecutable = Join-Path $resolvedPublishPath "QAReceiverRegistry.exe"
if (-not (Test-Path -LiteralPath $sourceExecutable)) {
    throw "Published service executable was not found: $sourceExecutable"
}

if (Get-NetTCPConnection -LocalPort $Port -ErrorAction SilentlyContinue) {
    throw "TCP port $Port is already in use."
}

$normalizedThumbprint = ""
$certificate = $null
if ($UseHttps) {
    $normalizedThumbprint = ($CertificateThumbprint -replace "[^0-9A-Fa-f]", "").ToUpperInvariant()
    if ($normalizedThumbprint.Length -ne 40) {
        throw "CertificateThumbprint must identify a certificate when HTTPS is enabled."
    }

    $certificate = Get-Item -LiteralPath "Cert:\LocalMachine\My\$normalizedThumbprint" -ErrorAction Stop
    if (-not $certificate.HasPrivateKey) {
        throw "The selected certificate does not have a private key."
    }
    if ($certificate.NotBefore -gt (Get-Date) -or $certificate.NotAfter -lt (Get-Date)) {
        throw "The selected certificate is not currently valid."
    }

    $ekuExtension = $certificate.Extensions |
        Where-Object { $_.Oid.Value -eq "2.5.29.37" } |
        Select-Object -First 1
    if ($ekuExtension) {
        $serverAuthenticationOid = "1.3.6.1.5.5.7.3.1"
        $ekuOids = @($ekuExtension.EnhancedKeyUsages | ForEach-Object { $_.Value })
        if ($serverAuthenticationOid -notin $ekuOids) {
            throw "The certificate cannot be used for HTTPS because its EKU does not include Server Authentication ($serverAuthenticationOid)."
        }
    }

    $certificateDnsNames = @($certificate.DnsNameList | ForEach-Object { $_.Unicode })
    if ($PublicHostName -notin $certificateDnsNames) {
        throw "The certificate SAN does not contain $PublicHostName."
    }
}

$appPath = Join-Path $RootPath "app"
$configPath = Join-Path $RootPath "config\appsettings.json"
$dataPath = Join-Path $RootPath "data"
$cloneFilesPath = Join-Path $RootPath "clone-files"
$logsPath = Join-Path $RootPath "logs"
@($RootPath, $appPath, (Split-Path $configPath), $dataPath, $cloneFilesPath, $logsPath, $BackupPath) |
    ForEach-Object { New-Item -ItemType Directory -Path $_ -Force | Out-Null }

Grant-DirectoryAccess -Path $RootPath -Permission RX
Grant-DirectoryAccess -Path $appPath -Permission RX
Grant-DirectoryAccess -Path (Split-Path $configPath) -Permission RX
Grant-DirectoryAccess -Path $dataPath -Permission M
Grant-DirectoryAccess -Path $cloneFilesPath -Permission M
Grant-DirectoryAccess -Path $logsPath -Permission M
Grant-DirectoryAccess -Path $BackupPath -Permission M

Copy-Item -Path (Join-Path $resolvedPublishPath "*") -Destination $appPath -Recurse -Force

$configuration = [ordered]@{
    Server = [ordered]@{
        ListenAddress = $ListenAddress
        Port = $Port
        UseHttps = [bool]$UseHttps
        CertificateThumbprint = $normalizedThumbprint
    }
    Registry = [ordered]@{
        DatabasePath = (Join-Path $dataPath "receivers.db")
        CloneFilesPath = $cloneFilesPath
        BackupPath = $BackupPath
        BusyTimeoutSeconds = 15
        BackupIntervalHours = 24
        BackupRetentionCount = 14
        BackupOnStartup = $true
    }
    Logging = [ordered]@{
        LogLevel = [ordered]@{
            Default = "Information"
            "Microsoft.AspNetCore" = "Warning"
        }
    }
}
$configuration | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $configPath -Encoding utf8

if ($UseHttps) {
    Grant-CertificatePrivateKeyRead -Certificate $certificate
}

if (-not [System.Diagnostics.EventLog]::SourceExists($serviceDisplayName)) {
    New-EventLog -LogName Application -Source $serviceDisplayName
}

$executablePath = Join-Path $appPath "QAReceiverRegistry.exe"
& $executablePath --config $configPath --migrate-only
if ($LASTEXITCODE -ne 0) {
    throw "Database migration failed with exit code $LASTEXITCODE."
}

$binaryPath = "`"$executablePath`" --config `"$configPath`""
New-Service -Name $serviceName -BinaryPathName $binaryPath -DisplayName $serviceDisplayName -StartupType Automatic | Out-Null
& sc.exe config $serviceName obj= "NT AUTHORITY\LocalService" | Out-Host
if ($LASTEXITCODE -ne 0) {
    throw "Could not configure the service account."
}
& sc.exe failure $serviceName reset= 86400 actions= restart/5000/restart/15000/restart/60000 | Out-Host
& sc.exe failureflag $serviceName 1 | Out-Host

if (-not (Get-NetFirewallRule -DisplayName $firewallRuleName -ErrorAction SilentlyContinue)) {
    New-NetFirewallRule `
        -DisplayName $firewallRuleName `
        -Direction Inbound `
        -Action Allow `
        -Protocol TCP `
        -LocalPort $Port `
        -LocalAddress $ListenAddress `
        -Profile Domain | Out-Null
}

Start-Service -Name $serviceName
$deadline = (Get-Date).AddSeconds(30)
do {
    Start-Sleep -Seconds 1
    $service = Get-Service -Name $serviceName
} until ($service.Status -eq "Running" -or (Get-Date) -ge $deadline)

if ($service.Status -ne "Running") {
    throw "The service did not reach the Running state. Check the Windows Application event log."
}

$scheme = if ($UseHttps) { "https" } else { "http" }
$healthUri = "$scheme`://$PublicHostName`:$Port/health"
$health = Invoke-RestMethod -Uri $healthUri -Method Get
if ($health.status -ne "healthy") {
    throw "The service started, but its health check did not report healthy."
}

Write-Host "QA Receiver Registry is installed and healthy."
Write-Host "Health endpoint: $healthUri"
