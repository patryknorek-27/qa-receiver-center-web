[CmdletBinding()]
param(
    [string]$HostName = "HHKNQA1-DE-DE04.eu.trimblecorp.net",
    [ValidateRange(1024, 65535)]
    [int]$Port = 8787,
    [switch]$UseHttps,
    [switch]$TestIpManagement
)

$ErrorActionPreference = "Stop"
$scheme = if ($UseHttps) { "https" } else { "http" }
$uri = "$scheme`://$HostName`:$Port/health"
$response = Invoke-RestMethod -Uri $uri -Method Get

if ($response.status -ne "healthy" -or $response.database.integrityStatus -ne "ok") {
    $response | ConvertTo-Json -Depth 8
    throw "QA Receiver Registry health check failed."
}

$ipManagementStatus = "Not tested"
$automaticIpScan = "Not tested"
if ($TestIpManagement) {
    $inventory = Invoke-RestMethod `
        -Uri "$scheme`://$HostName`:$Port/api/v1/ip-addresses" `
        -Method Get `
        -Headers @{ "X-QA-User" = "$env:USERDOMAIN\$env:USERNAME" }
    if (@($inventory).Count -ne 254) {
        throw "IP Management API returned $(@($inventory).Count) addresses instead of 254."
    }
    $monitoring = Invoke-RestMethod `
        -Uri "$scheme`://$HostName`:$Port/api/v1/settings/ip-monitoring" `
        -Method Get
    if ($monitoring.value.intervalMinutes -lt 5 -or $monitoring.value.timeoutMilliseconds -lt 250) {
        throw "IP Monitoring settings returned invalid values."
    }
    $ipManagementStatus = "API verified"
    $automaticIpScan = if ($monitoring.value.automaticScanEnabled) {
        "Every $($monitoring.value.intervalMinutes) minutes, timeout $($monitoring.value.timeoutMilliseconds) ms"
    }
    else {
        "Disabled, manual ping available"
    }
}

[pscustomobject]@{
    Status = $response.status
    ServiceVersion = $response.serviceVersion
    SqliteVersion = $response.database.sqliteVersion
    SchemaVersion = $response.database.schemaVersion
    ReceiverCount = $response.database.receiverCount
    IpAddressCount = $response.database.ipAddressCount
    PendingIpRequestCount = $response.database.pendingIpRequestCount
    IpManagement = $ipManagementStatus
    AutomaticIpScan = $automaticIpScan
    Integrity = $response.database.integrityStatus
    Uri = $uri
} | Format-List
