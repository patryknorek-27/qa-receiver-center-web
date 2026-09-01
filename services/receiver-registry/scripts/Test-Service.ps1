[CmdletBinding()]
param(
    [string]$HostName = "HHKNQA1-DE-DE04.eu.trimblecorp.net",
    [ValidateRange(1024, 65535)]
    [int]$Port = 8787,
    [switch]$UseHttps
)

$ErrorActionPreference = "Stop"
$scheme = if ($UseHttps) { "https" } else { "http" }
$uri = "$scheme`://$HostName`:$Port/health"
$response = Invoke-RestMethod -Uri $uri -Method Get

if ($response.status -ne "healthy" -or $response.database.integrityStatus -ne "ok") {
    $response | ConvertTo-Json -Depth 8
    throw "QA Receiver Registry health check failed."
}

[pscustomobject]@{
    Status = $response.status
    ServiceVersion = $response.serviceVersion
    SqliteVersion = $response.database.sqliteVersion
    SchemaVersion = $response.database.schemaVersion
    ReceiverCount = $response.database.receiverCount
    Integrity = $response.database.integrityStatus
    Uri = $uri
} | Format-List
