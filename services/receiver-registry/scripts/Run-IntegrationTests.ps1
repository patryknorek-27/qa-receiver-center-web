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
$ipAdminHeaders = @{
    "X-QA-User" = "integration-test"
}
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
        Features = [ordered]@{
            IpManagementEnabled = $true
            GuestPortalEnabled = $true
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

    if (-not $health -or $health.status -ne "healthy" -or $health.database.schemaVersion -ne 8 -or
        $health.database.ipAddressCount -ne 254 -or $health.database.pendingIpRequestCount -ne 0) {
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
    $receiverAudit = @($audit | Where-Object {
        $_.entityType -eq "receiver" -and $_.entityId -eq $receiver.id -and $_.actor -eq "integration-test"
    })
    if ($receiverAudit.Count -ne 2 -or "created" -notin $receiverAudit.action -or "updated" -notin $receiverAudit.action) {
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

    $ipMonitoringSetting = Invoke-RestMethod -Uri "$baseUri/api/v1/settings/ip-monitoring"
    if ($ipMonitoringSetting.rowVersion -ne 1 -or
        -not $ipMonitoringSetting.value.automaticScanEnabled -or
        $ipMonitoringSetting.value.intervalMinutes -ne 10 -or
        $ipMonitoringSetting.value.timeoutMilliseconds -ne 1000) {
        throw "Default IP monitoring setting was not created correctly."
    }

    $ipMonitoringBody = @{
        value = @{
            automaticScanEnabled = $true
            intervalMinutes = 15
            timeoutMilliseconds = 2000
        }
        rowVersion = $ipMonitoringSetting.rowVersion
    } | ConvertTo-Json -Depth 8
    $updatedIpMonitoringSetting = Invoke-RestMethod `
        -Method Put `
        -Uri "$baseUri/api/v1/settings/ip-monitoring" `
        -Headers @{ "X-QA-User" = "integration-test" } `
        -ContentType "application/json" `
        -Body $ipMonitoringBody
    if ($updatedIpMonitoringSetting.rowVersion -ne 2 -or
        $updatedIpMonitoringSetting.value.intervalMinutes -ne 15) {
        throw "IP monitoring setting update returned unexpected data."
    }

    $invalidIpMonitoringStatus = $null
    $invalidIpMonitoringBody = @{
        value = @{
            automaticScanEnabled = $true
            intervalMinutes = 1
            timeoutMilliseconds = 1000
        }
        rowVersion = $updatedIpMonitoringSetting.rowVersion
    } | ConvertTo-Json -Depth 8
    try {
        Invoke-RestMethod `
            -Method Put `
            -Uri "$baseUri/api/v1/settings/ip-monitoring" `
            -Headers @{ "X-QA-User" = "integration-test" } `
            -ContentType "application/json" `
            -Body $invalidIpMonitoringBody | Out-Null
    }
    catch {
        $invalidIpMonitoringStatus = [int]$_.Exception.Response.StatusCode
    }
    if ($invalidIpMonitoringStatus -ne 400) {
        throw "Invalid IP monitoring interval did not return HTTP 400 Bad Request."
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

    $guestIpInventory = Invoke-RestMethod -Uri "$baseUri/api/v1/guest/ip-addresses"
    if ($guestIpInventory.Count -ne 254 -or
        $guestIpInventory[1].PSObject.Properties.Name -contains "isOnline" -or
        $guestIpInventory[1].PSObject.Properties.Name -contains "lastPingAt" -or
        $guestIpInventory[1].PSObject.Properties.Name -contains "rowVersion" -or
        -not ($guestIpInventory[1].PSObject.Properties.Name -contains "usageEndDate") -or
        -not ($guestIpInventory[1].PSObject.Properties.Name -contains "antenna1") -or
        -not ($guestIpInventory[1].PSObject.Properties.Name -contains "antenna2") -or
        -not ($guestIpInventory[1].PSObject.Properties.Name -contains "requestAvailable")) {
        throw "Guest IP inventory contains missing records or protected fields."
    }

    $guestPage = Invoke-WebRequest -Uri "$baseUri/" -UseBasicParsing
    if ($guestPage.StatusCode -ne 200 -or
        $guestPage.Content -notmatch "QA 155 LAN Monitoring" -or
        $guestPage.Headers["Content-Security-Policy"] -notmatch "default-src 'self'") {
        throw "Guest portal page or its security headers are not available."
    }
    $legacyGuestPage = Invoke-WebRequest -Uri "$baseUri/guest/" -UseBasicParsing
    if ($legacyGuestPage.StatusCode -ne 200 -or
        $legacyGuestPage.Content -notmatch "QA 155 LAN Monitoring") {
        throw "The previous guest portal URL is no longer available."
    }
    $guestScript = Invoke-WebRequest -Uri "$baseUri/guest/guest.js" -UseBasicParsing
    if ($guestScript.StatusCode -ne 200 -or $guestScript.Content -notmatch "/api/v1/guest/ip-addresses") {
        throw "Guest portal script is not available."
    }
    $unusedGuestRecords = @($guestIpInventory | Where-Object requestAvailable)
    $usedGuestRecords = @($guestIpInventory | Where-Object status -eq "Used")
    if ($unusedGuestRecords.Count -ne 85 -or $usedGuestRecords.Count -ne 168) {
        throw "The initial IP inventory seed does not match Static IP Overview.xlsx."
    }
    $requestIp = $unusedGuestRecords[0].ipAddress
    $rejectIp = $unusedGuestRecords[1].ipAddress
    $updateIp = $unusedGuestRecords[2].ipAddress
    $bulkChangeIp = $unusedGuestRecords[3].ipAddress
    $conflictProbeIp = $unusedGuestRecords[4].ipAddress
    $usageEndDate = (Get-Date).Date.AddDays(30).ToString("yyyy-MM-dd")

    $guestRequestBody = @{
        ipAddress = $requestIp
        requestedBy = "Guest User"
        location = "QA Lab"
        responsible = "Test Owner"
        environment = "Testing/Lab"
        deviceType = "Alloy"
        serialNumber = "REQUEST-001"
        antenna1 = "ANT-REQUEST-01"
        antenna2 = "ANT-REQUEST-02"
        notes = "Integration request"
        usageEndDate = $usageEndDate
    } | ConvertTo-Json
    $guestRequest = Invoke-RestMethod `
        -Method Post `
        -Uri "$baseUri/api/v1/guest/ip-requests" `
        -ContentType "application/json" `
        -Body $guestRequestBody
    if ($guestRequest.status -ne "Pending" -or
        $guestRequest.PSObject.Properties.Name -contains "rowVersion" -or
        $guestRequest.PSObject.Properties.Name -contains "decisionBy") {
        throw "Guest IP request returned unexpected data."
    }

    $duplicateRequestStatus = $null
    try {
        Invoke-RestMethod `
            -Method Post `
            -Uri "$baseUri/api/v1/guest/ip-requests" `
            -ContentType "application/json" `
            -Body $guestRequestBody | Out-Null
    }
    catch {
        $duplicateRequestStatus = [int]$_.Exception.Response.StatusCode
    }
    if ($duplicateRequestStatus -ne 409) {
        throw "A second pending request for the same IP did not return HTTP 409 Conflict."
    }

    $guestInventoryAfterRequest = Invoke-RestMethod -Uri "$baseUri/api/v1/guest/ip-addresses"
    $requestedGuestIp = $guestInventoryAfterRequest | Where-Object ipAddress -eq $requestIp
    if (-not $requestedGuestIp.requestPending -or $requestedGuestIp.requestAvailable) {
        throw "A pending request did not remove the IP address from guest availability."
    }

    $pendingRequests = Invoke-RestMethod `
        -Uri "$baseUri/api/v1/ip-requests?status=Pending" `
        -Headers $ipAdminHeaders
    if (@($pendingRequests).Count -ne 1 -or $pendingRequests.id -ne $guestRequest.id -or
        $pendingRequests.antenna1 -ne "ANT-REQUEST-01" -or
        $pendingRequests.antenna2 -ne "ANT-REQUEST-02" -or
        $pendingRequests.usageEndDate -ne $usageEndDate) {
        throw "Administrative request list returned unexpected data."
    }
    $adminGuestRequest = @($pendingRequests)[0]

    $ipInventory = Invoke-RestMethod `
        -Uri "$baseUri/api/v1/ip-addresses" `
        -Headers $ipAdminHeaders
    if ($ipInventory.Count -ne 254 -or $ipInventory[0].ipAddress -ne "10.2.155.1" -or
        $ipInventory[0].status -ne "Reserved") {
        throw "Initial IP inventory returned unexpected data."
    }
    $requestIpRecord = $ipInventory | Where-Object ipAddress -eq $requestIp
    $updateIpRecord = $ipInventory | Where-Object ipAddress -eq $updateIp

    $approveRequestBody = @{
        location = $adminGuestRequest.location
        responsible = $adminGuestRequest.responsible
        environment = $adminGuestRequest.environment
        deviceType = $adminGuestRequest.deviceType
        serialNumber = $adminGuestRequest.serialNumber
        antenna1 = $adminGuestRequest.antenna1
        antenna2 = $adminGuestRequest.antenna2
        notes = $adminGuestRequest.notes
        usageEndDate = $adminGuestRequest.usageEndDate
        requestRowVersion = $adminGuestRequest.rowVersion
        ipRowVersion = $requestIpRecord.rowVersion
    } | ConvertTo-Json
    $approvedRequest = Invoke-RestMethod `
        -Method Post `
        -Uri "$baseUri/api/v1/ip-requests/$($guestRequest.id)/approve" `
        -Headers $ipAdminHeaders `
        -ContentType "application/json" `
        -Body $approveRequestBody
    if ($approvedRequest.status -ne "Approved" -or $approvedRequest.rowVersion -ne 2 -or
        $approvedRequest.decisionBy -ne "integration-test") {
        throw "IP request approval returned unexpected data."
    }

    $approvedEvents = Invoke-RestMethod `
        -Uri "$baseUri/api/v1/ip-requests/$($guestRequest.id)/events" `
        -Headers $ipAdminHeaders
    if (@($approvedEvents).Count -ne 2 -or $approvedEvents[0].action -ne "Created" -or
        $approvedEvents[1].action -ne "Approved") {
        throw "IP request event history returned unexpected data."
    }

    $approvedIp = Invoke-RestMethod `
        -Uri "$baseUri/api/v1/ip-addresses/$requestIp" `
        -Headers $ipAdminHeaders
    if ($approvedIp.status -ne "Used" -or $approvedIp.serialNumber -ne "REQUEST-001" -or
        $approvedIp.antenna1 -ne "ANT-REQUEST-01" -or
        $approvedIp.antenna2 -ne "ANT-REQUEST-02" -or
        $approvedIp.usageEndDate -ne $usageEndDate -or
        $approvedIp.rowVersion -ne ($requestIpRecord.rowVersion + 1)) {
        throw "Approved request did not assign the IP inventory record."
    }

    $rejectRequestBody = @{
        ipAddress = $rejectIp
        requestedBy = "Second Guest"
        responsible = "Second Guest"
        notes = "Request to reject"
    } | ConvertTo-Json
    $requestToReject = Invoke-RestMethod `
        -Method Post `
        -Uri "$baseUri/api/v1/guest/ip-requests" `
        -ContentType "application/json" `
        -Body $rejectRequestBody
    $adminRequestToReject = Invoke-RestMethod `
        -Uri "$baseUri/api/v1/ip-requests/$($requestToReject.id)" `
        -Headers $ipAdminHeaders
    $rejectionBody = @{
        reason = "Address is reserved for another test."
        rowVersion = $adminRequestToReject.rowVersion
    } | ConvertTo-Json
    $rejectedRequest = Invoke-RestMethod `
        -Method Post `
        -Uri "$baseUri/api/v1/ip-requests/$($requestToReject.id)/reject" `
        -Headers $ipAdminHeaders `
        -ContentType "application/json" `
        -Body $rejectionBody
    if ($rejectedRequest.status -ne "Rejected" -or
        $rejectedRequest.decisionReason -ne "Address is reserved for another test.") {
        throw "IP request rejection returned unexpected data."
    }

    $ipUpdateBody = @{
        status = "Used"
        location = "QA Lab"
        responsible = "Integration Test"
        environment = "Testing/Lab"
        deviceType = "Alloy"
        serialNumber = "IP-TEST-001"
        antenna1 = "ANT-IP-01"
        antenna2 = "ANT-IP-02"
        notes = "IP management integration test"
        usageEndDate = $null
        rowVersion = $updateIpRecord.rowVersion
    } | ConvertTo-Json
    $updatedIp = Invoke-RestMethod `
        -Method Put `
        -Uri "$baseUri/api/v1/ip-addresses/$updateIp" `
        -Headers $ipAdminHeaders `
        -ContentType "application/json" `
        -Body $ipUpdateBody
    if ($updatedIp.status -ne "Used" -or $updatedIp.rowVersion -ne ($updateIpRecord.rowVersion + 1) -or
        $updatedIp.antenna1 -ne "ANT-IP-01" -or $updatedIp.antenna2 -ne "ANT-IP-02" -or
        $updatedIp.environment -ne "Testing/Lab" -or $updatedIp.usageEndDate -ne $null -or
        $updatedIp.updatedBy -ne "integration-test") {
        throw "IP inventory update returned unexpected data."
    }

    $ipConflictStatus = $null
    try {
        Invoke-RestMethod `
            -Method Put `
            -Uri "$baseUri/api/v1/ip-addresses/$updateIp" `
            -Headers $ipAdminHeaders `
            -ContentType "application/json" `
            -Body $ipUpdateBody | Out-Null
    }
    catch {
        $ipConflictStatus = [int]$_.Exception.Response.StatusCode
    }
    if ($ipConflictStatus -ne 409) {
        throw "Stale IP inventory update did not return HTTP 409 Conflict."
    }

    $gatewayUpdateBody = @{
        status = "Unused"
        rowVersion = $ipInventory[0].rowVersion
    } | ConvertTo-Json
    $gatewayStatus = $null
    try {
        Invoke-RestMethod `
            -Method Put `
            -Uri "$baseUri/api/v1/ip-addresses/10.2.155.1" `
            -Headers $ipAdminHeaders `
            -ContentType "application/json" `
            -Body $gatewayUpdateBody | Out-Null
    }
    catch {
        $gatewayStatus = [int]$_.Exception.Response.StatusCode
    }
    if ($gatewayStatus -ne 400) {
        throw "The gateway IP could be changed from Reserved."
    }

    $ipAudit = Invoke-RestMethod -Uri "$baseUri/api/v1/audit?limit=1"
    if ($ipAudit.entityType -ne "ip-address" -or $ipAudit.entityId -ne $updateIp -or
        $ipAudit.actor -ne "integration-test") {
        throw "IP inventory audit record was not written correctly."
    }

    $beforeBulkImport = Invoke-RestMethod `
        -Uri "$baseUri/api/v1/ip-addresses" `
        -Headers $ipAdminHeaders
    $bulkItems = @($beforeBulkImport | Where-Object sortOrder -gt 1 | ForEach-Object {
        @{
            ipAddress = $_.ipAddress
            status = if ($_.ipAddress -eq $bulkChangeIp) { "Used" } else { $_.status }
            location = $_.location
            responsible = $_.responsible
            environment = $_.environment
            deviceType = $_.deviceType
            serialNumber = $_.serialNumber
            antenna1 = $_.antenna1
            antenna2 = $_.antenna2
            notes = $_.notes
            usageEndDate = $_.usageEndDate
            rowVersion = $_.rowVersion
        }
    })
    if ($bulkItems.Count -ne 253) {
        throw "Bulk import test created $($bulkItems.Count) records instead of 253."
    }
    $bulkImportBody = @{ items = $bulkItems } | ConvertTo-Json -Depth 8
    $bulkImportBodyBytes = [Text.Encoding]::UTF8.GetBytes($bulkImportBody)
    try {
        $bulkImport = Invoke-RestMethod `
            -Method Put `
            -Uri "$baseUri/api/v1/ip-addresses/import" `
            -Headers $ipAdminHeaders `
            -ContentType "application/json" `
            -Body $bulkImportBodyBytes
    }
    catch {
        $responseStatus = $null
        $responseText = $_.ErrorDetails.Message
        if (-not $responseText -and $_.Exception.Response) {
            $responseStatus = [int]$_.Exception.Response.StatusCode
            if ($_.Exception.Response.Content) {
                $responseText = $_.Exception.Response.Content.ReadAsStringAsync().GetAwaiter().GetResult()
            }
            elseif ($_.Exception.Response.PSObject.Methods.Name -contains "GetResponseStream") {
                $responseStream = $_.Exception.Response.GetResponseStream()
                $responseReader = [System.IO.StreamReader]::new($responseStream)
                try {
                    $responseText = $responseReader.ReadToEnd()
                }
                finally {
                    $responseReader.Dispose()
                    $responseStream.Dispose()
                }
            }
        }
        $serviceLog = if (Test-Path -LiteralPath $stdoutPath) {
            (Get-Content -LiteralPath $stdoutPath -Tail 20) -join [Environment]::NewLine
        }
        else {
            "Service log was not found."
        }
        throw "Atomic IP inventory import failed with HTTP $responseStatus. $responseText $($_.Exception.Message)`n$serviceLog"
    }
    if ($bulkImport.updatedCount -ne 253 -or
        -not (Test-Path -LiteralPath $bulkImport.backup.fullPath) -or
        $bulkImport.backup.sizeBytes -le 0) {
        throw "Atomic IP inventory import did not create a verified backup or update all records."
    }

    $afterBulkImport = Invoke-RestMethod `
        -Uri "$baseUri/api/v1/ip-addresses" `
        -Headers $ipAdminHeaders
    $importedIp = $afterBulkImport | Where-Object ipAddress -eq $bulkChangeIp
    if ($importedIp.status -ne "Used") {
        throw "Atomic IP inventory import did not apply the expected value."
    }

    $beforeRollbackCheck = $afterBulkImport | Where-Object ipAddress -eq $updateIp
    $conflictingBulkItems = @($afterBulkImport | Where-Object sortOrder -gt 1 | ForEach-Object {
        @{
            ipAddress = $_.ipAddress
            status = $_.status
            location = $_.location
            responsible = $_.responsible
            environment = $_.environment
            deviceType = $_.deviceType
            serialNumber = $_.serialNumber
            antenna1 = $_.antenna1
            antenna2 = $_.antenna2
            notes = $_.notes
            usageEndDate = $_.usageEndDate
            rowVersion = if ($_.ipAddress -eq $conflictProbeIp) { $_.rowVersion - 1 } else { $_.rowVersion }
        }
    })
    $conflictingBulkStatus = $null
    try {
        Invoke-RestMethod `
            -Method Put `
            -Uri "$baseUri/api/v1/ip-addresses/import" `
            -Headers $ipAdminHeaders `
            -ContentType "application/json" `
            -Body ([Text.Encoding]::UTF8.GetBytes(
                (@{ items = $conflictingBulkItems } | ConvertTo-Json -Depth 8))) | Out-Null
    }
    catch {
        $conflictingBulkStatus = [int]$_.Exception.Response.StatusCode
    }
    if ($conflictingBulkStatus -ne 409) {
        throw "Conflicting IP inventory import did not return HTTP 409 Conflict."
    }
    $afterRollbackCheck = Invoke-RestMethod `
        -Uri "$baseUri/api/v1/ip-addresses/$updateIp" `
        -Headers $ipAdminHeaders
    if ($afterRollbackCheck.rowVersion -ne $beforeRollbackCheck.rowVersion) {
        throw "Conflicting IP inventory import was not rolled back completely."
    }

    & (Join-Path $serviceRoot "scripts\Test-Service.ps1") `
        -HostName "127.0.0.1" `
        -Port $Port `
        -TestIpManagement

    $backup = Invoke-RestMethod -Method Post -Uri "$baseUri/api/v1/maintenance/backup"
    if (-not (Test-Path -LiteralPath $backup.fullPath) -or $backup.sizeBytes -le 0) {
        throw "Verified database backup was not created."
    }

    Write-Host "Integration tests passed."
    Write-Host "Health, migrations, receiver CRUD, IP inventory, shared settings, firmware history, Clone Files, audit, import, and backup are working."
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
