[CmdletBinding()]
param(
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Debug",
    [ValidateRange(1024, 65535)]
    [int]$Port = 18787
)

$ErrorActionPreference = "Stop"
$serviceRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$projectPath = Join-Path $serviceRoot "QAReceiverRegistry.csproj"
$nugetConfig = Join-Path $serviceRoot "NuGet.Config"
$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) "qa-receiver-registry-tests-$([guid]::NewGuid())"
$configPath = Join-Path $testRoot "appsettings.test.json"
$importSourcePath = Join-Path $testRoot "qa-receivers.test.json"
$stdoutPath = Join-Path $testRoot "service.stdout.log"
$stderrPath = Join-Path $testRoot "service.stderr.log"
$baseUri = "http://127.0.0.1:$Port"
$process = $null

try {
    New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
    dotnet build $projectPath --configuration $Configuration --configfile $nugetConfig
    if ($LASTEXITCODE -ne 0) {
        throw "Service build failed with exit code $LASTEXITCODE."
    }

    $targetFramework = "net10.0-windows"
    $executablePath = Join-Path $serviceRoot "bin\$Configuration\$targetFramework\win-x64\QAReceiverRegistry.exe"
    if (-not (Test-Path -LiteralPath $executablePath)) {
        throw "Service executable was not found: $executablePath"
    }

    $serviceConfig = [ordered]@{
        Server = [ordered]@{
            ListenAddress = "127.0.0.1"
            Port = $Port
            UseHttps = $false
            CertificateThumbprint = ""
        }
        Registry = [ordered]@{
            DatabasePath = (Join-Path $testRoot "data\receivers.db")
            CloneFilesPath = (Join-Path $testRoot "clone-files")
            BackupPath = (Join-Path $testRoot "backups")
            BusyTimeoutSeconds = 5
            BackupIntervalHours = 24
            BackupRetentionCount = 3
            BackupOnStartup = $false
        }
        Logging = [ordered]@{
            LogLevel = [ordered]@{
                Default = "Warning"
                "Microsoft.AspNetCore" = "Warning"
            }
        }
    }
    $serviceConfig | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $configPath -Encoding utf8

    $process = Start-Process `
        -FilePath $executablePath `
        -ArgumentList @("--config", $configPath) `
        -PassThru `
        -WindowStyle Hidden `
        -RedirectStandardOutput $stdoutPath `
        -RedirectStandardError $stderrPath

    $deadline = (Get-Date).AddSeconds(30)
    do {
        if ($process.HasExited) {
            throw "Service exited during startup. See $stderrPath"
        }

        try {
            $health = Invoke-RestMethod -Uri "$baseUri/health" -TimeoutSec 2
        }
        catch {
            $health = $null
            Start-Sleep -Milliseconds 250
        }
    } until ($health -or (Get-Date) -ge $deadline)

    if (-not $health -or $health.status -ne "healthy" -or $health.database.schemaVersion -ne 3) {
        throw "Service did not become healthy within 30 seconds."
    }

    $groups = Invoke-RestMethod -Uri "$baseUri/api/v1/groups"
    if ($groups.Count -ne 4 -or $groups[0].code -ne "BETA" -or $groups[1].code -ne "ALPHA") {
        throw "Initial receiver groups were not created correctly."
    }

    $createBody = @{
        ipAddress = "10.2.155.11"
        name = "BX1092i"
        model = "stella"
        serialNumber = "TEST-001"
        location = "QA Lab"
        notes = "Integration test"
        enabled = $true
        groupIds = @(1)
        primaryGroupId = 1
    } | ConvertTo-Json
    $receiver = Invoke-RestMethod `
        -Method Post `
        -Uri "$baseUri/api/v1/receivers" `
        -Headers @{ "X-QA-User" = "integration-test" } `
        -ContentType "application/json" `
        -Body $createBody

    if ($receiver.rowVersion -ne 1 -or $receiver.groups[0].code -ne "BETA") {
        throw "Receiver creation returned unexpected data."
    }

    $updateBody = @{
        ipAddress = $receiver.ipAddress
        name = "BX1092i updated"
        model = $receiver.model
        serialNumber = $receiver.serialNumber
        location = $receiver.location
        notes = "Updated integration test"
        enabled = $true
        groupIds = @(1, 2)
        primaryGroupId = 1
        rowVersion = $receiver.rowVersion
    } | ConvertTo-Json
    $updated = Invoke-RestMethod `
        -Method Put `
        -Uri "$baseUri/api/v1/receivers/$($receiver.id)" `
        -Headers @{ "X-QA-User" = "integration-test" } `
        -ContentType "application/json" `
        -Body $updateBody

    if ($updated.rowVersion -ne 2 -or $updated.groups.Count -ne 2) {
        throw "Receiver update or group membership replacement failed."
    }

    $conflictStatus = $null
    try {
        Invoke-RestMethod `
            -Method Put `
            -Uri "$baseUri/api/v1/receivers/$($receiver.id)" `
            -Headers @{ "X-QA-User" = "integration-test" } `
            -ContentType "application/json" `
            -Body $updateBody | Out-Null
    }
    catch {
        $conflictStatus = [int]$_.Exception.Response.StatusCode
    }

    if ($conflictStatus -ne 409) {
        throw "Stale update did not return HTTP 409 Conflict."
    }

    $audit = Invoke-RestMethod -Uri "$baseUri/api/v1/audit?limit=10"
    if ($audit.Count -ne 2 -or $audit[0].action -ne "updated" -or $audit[0].actor -ne "integration-test") {
        throw "Audit records were not written correctly."
    }

    $setting = Invoke-RestMethod -Uri "$baseUri/api/v1/settings/update-safety"
    if ($setting.rowVersion -ne 1 -or $setting.value.restartCycleMinutes -ne 30) {
        throw "Default shared setting was not created correctly."
    }

    $settingBody = @{
        value = @{
            postRestartDelayMinutes = 3
            preRestartSafetyMarginMinutes = 9
            restartCycleMinutes = 30
        }
        rowVersion = $setting.rowVersion
    } | ConvertTo-Json -Depth 8
    $updatedSetting = Invoke-RestMethod `
        -Method Put `
        -Uri "$baseUri/api/v1/settings/update-safety" `
        -Headers @{ "X-QA-User" = "integration-test" } `
        -ContentType "application/json" `
        -Body $settingBody
    if ($updatedSetting.rowVersion -ne 2 -or $updatedSetting.value.postRestartDelayMinutes -ne 3) {
        throw "Shared setting update returned unexpected data."
    }

    $settingConflictStatus = $null
    try {
        Invoke-RestMethod `
            -Method Put `
            -Uri "$baseUri/api/v1/settings/update-safety" `
            -Headers @{ "X-QA-User" = "integration-test" } `
            -ContentType "application/json" `
            -Body $settingBody | Out-Null
    }
    catch {
        $settingConflictStatus = [int]$_.Exception.Response.StatusCode
    }
    if ($settingConflictStatus -ne 409) {
        throw "Stale shared setting update did not return HTTP 409 Conflict."
    }

    $settingAudit = Invoke-RestMethod -Uri "$baseUri/api/v1/audit?limit=1"
    if ($settingAudit.entityType -ne "setting" -or $settingAudit.entityId -ne "update-safety") {
        throw "Shared setting audit record was not written correctly."
    }

    $historyJobId = "integration-history-job"
    $historyBody = @{
        batchId = "integration-batch"
        operationType = "multiple"
        groupName = $null
        ip = "10.2.155.11"
        receiverName = "BX1092i updated"
        model = "stella"
        targetFile = "FW_6.70-0.26.timg"
        startedAt = "1788255216"
        completedAt = "1788255793"
        result = "Completed"
        finalFirmwareVersion = "6.70-0.26"
        errors = @()
    } | ConvertTo-Json -Depth 8
    $historyEntry = Invoke-RestMethod `
        -Method Put `
        -Uri "$baseUri/api/v1/firmware-history/$historyJobId" `
        -Headers @{ "X-QA-User" = "integration-test" } `
        -ContentType "application/json" `
        -Body $historyBody
    if ($historyEntry.rowVersion -ne 1 -or $historyEntry.ip -ne "10.2.155.11" -or
        $historyEntry.startedAt -ne "1788255216" -or $historyEntry.cloneFile) {
        throw "Firmware history creation returned unexpected data."
    }

    $unchangedHistoryEntry = Invoke-RestMethod `
        -Method Put `
        -Uri "$baseUri/api/v1/firmware-history/$historyJobId" `
        -Headers @{ "X-QA-User" = "integration-test" } `
        -ContentType "application/json" `
        -Body $historyBody
    if ($unchangedHistoryEntry.rowVersion -ne 1) {
        throw "Idempotent firmware history synchronization changed the row version."
    }

    $cloneSourcePath = Join-Path $testRoot "integration-clone.xml"
    $cloneDownloadPath = Join-Path $testRoot "downloaded-clone.xml"
    '<CloneConfigData><Receiver>integration</Receiver></CloneConfigData>' |
        Set-Content -LiteralPath $cloneSourcePath -Encoding utf8
    $cloneFile = Invoke-RestMethod `
        -Method Put `
        -Uri "$baseUri/api/v1/firmware-history/$historyJobId/clone-file?fileName=integration-clone.xml" `
        -Headers @{ "X-QA-User" = "integration-test" } `
        -ContentType "application/xml" `
        -InFile $cloneSourcePath
    if ($cloneFile.fileName -ne "integration-clone.xml" -or $cloneFile.sizeBytes -le 0 -or $cloneFile.sha256.Length -ne 64) {
        throw "Clone File upload returned unexpected data."
    }

    $historyList = Invoke-RestMethod -Uri "$baseUri/api/v1/firmware-history"
    if ($historyList.Count -ne 1 -or $historyList[0].cloneFile.sha256 -ne $cloneFile.sha256) {
        throw "Shared firmware history did not include the uploaded Clone File."
    }

    Invoke-WebRequest `
        -Uri "$baseUri/api/v1/firmware-history/$historyJobId/clone-file" `
        -OutFile $cloneDownloadPath | Out-Null
    if ((Get-FileHash -LiteralPath $cloneDownloadPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $cloneFile.sha256) {
        throw "Downloaded Clone File checksum did not match the stored file."
    }

    Invoke-RestMethod `
        -Method Delete `
        -Uri "$baseUri/api/v1/firmware-history/$historyJobId" `
        -Headers @{ "X-QA-User" = "integration-test" } | Out-Null
    $historyAfterDelete = Invoke-RestMethod -Uri "$baseUri/api/v1/firmware-history"
    if ($null -ne $historyAfterDelete -and @($historyAfterDelete).Count -ne 0) {
        throw "Firmware history deletion did not remove the record."
    }

    $deletedHistoryStatus = $null
    try {
        Invoke-RestMethod `
            -Method Put `
            -Uri "$baseUri/api/v1/firmware-history/$historyJobId" `
            -Headers @{ "X-QA-User" = "integration-test" } `
            -ContentType "application/json" `
            -Body $historyBody | Out-Null
    }
    catch {
        $deletedHistoryStatus = [int]$_.Exception.Response.StatusCode
    }
    if ($deletedHistoryStatus -ne 409) {
        throw "A deleted firmware history entry was restored from stale data."
    }

    @{
        receivers = @(
            @{ addr = "10.2.155.11"; user = "qa"; pw = "secret"; group = 1 },
            @{ addr = "10.2.155.12"; user = "qa"; pw = "secret"; group = 2 }
        )
    } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $importSourcePath -Encoding utf8
    try {
        & (Join-Path $serviceRoot "scripts\Import-Receivers.ps1") `
            -RegistryBaseUri $baseUri `
            -SourcePath $importSourcePath `
            -Apply
    }
    catch {
        Write-Host $_.ScriptStackTrace
        throw
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Receiver import failed with exit code $LASTEXITCODE."
    }

    $importedReceivers = Invoke-RestMethod -Uri "$baseUri/api/v1/receivers"
    $metadataReceiver = $importedReceivers | Where-Object ipAddress -eq "10.2.155.12"
    if ($importedReceivers.Count -ne 2 -or @($metadataReceiver.groups).Count -ne 0) {
        throw "Receiver import returned unexpected data."
    }

    $backup = Invoke-RestMethod -Method Post -Uri "$baseUri/api/v1/maintenance/backup"
    if (-not (Test-Path -LiteralPath $backup.fullPath) -or $backup.sizeBytes -le 0) {
        throw "Verified database backup was not created."
    }

    Write-Host "Integration tests passed."
    Write-Host "Health, migrations, receiver CRUD, shared settings, firmware history, Clone Files, audit, import, and backup are working."
}
finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
        $process.WaitForExit()
    }

    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
