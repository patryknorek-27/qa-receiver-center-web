[CmdletBinding()]
param(
    [string]$RegistryBaseUri = "http://HHKNQA1-DE-DE04.eu.trimblecorp.net:8787",
    [string]$AdoApiUrl = "https://det-ados/tfs/Terrasat/QA/_apis/git/repositories/rx-monitoring-dhs/items?path=%2Fqa-receivers.json&versionDescriptor.version=main&versionDescriptor.versionType=branch&includeContent=true&api-version=7.0",
    [string]$SourcePath = "",
    [switch]$Apply
)

$ErrorActionPreference = "Stop"
$baseUri = $RegistryBaseUri.TrimEnd("/")
$actor = [Security.Principal.WindowsIdentity]::GetCurrent().Name
$headers = @{ "X-QA-User" = $actor }

function Get-ReceiverNodes {
    param([object]$Node)

    $result = [System.Collections.Generic.List[object]]::new()
    if ($null -eq $Node) {
        return $result
    }

    if ($Node -is [System.Collections.IEnumerable] -and $Node -isnot [string] -and $Node -isnot [pscustomobject]) {
        foreach ($item in $Node) {
            foreach ($receiver in @(Get-ReceiverNodes -Node $item)) {
                $result.Add($receiver) | Out-Null
            }
        }
        return $result
    }

    if ($Node -is [pscustomobject]) {
        $properties = $Node.PSObject.Properties
        $addrProperty = $properties["addr"]
        $userProperty = $properties["user"]
        $passwordProperty = $properties["pw"]
        $groupProperty = $properties["group"]

        if ($addrProperty -and $userProperty -and $passwordProperty -and $groupProperty) {
            $ip = [string]$addrProperty.Value
            $group = 0
            $validGroup = [int]::TryParse([string]$groupProperty.Value, [ref]$group) -and $group -ge 1 -and $group -le 4
            $parsedIp = $null
            $validIp = [System.Net.IPAddress]::TryParse($ip.Trim(), [ref]$parsedIp)
            if ($validIp -and $validGroup -and -not [string]::IsNullOrWhiteSpace([string]$userProperty.Value) -and -not [string]::IsNullOrWhiteSpace([string]$passwordProperty.Value)) {
                $result.Add([pscustomobject]@{
                    IpAddress = $ip.Trim()
                    GroupId = $group
                }) | Out-Null
            }
        }

        foreach ($property in $properties) {
            foreach ($receiver in @(Get-ReceiverNodes -Node $property.Value)) {
                $result.Add($receiver) | Out-Null
            }
        }
    }

    return $result
}

function Get-SourceJson {
    if (-not [string]::IsNullOrWhiteSpace($SourcePath)) {
        return Get-Content -LiteralPath $SourcePath -Raw
    }

    $response = Invoke-WebRequest `
        -UseBasicParsing `
        -UseDefaultCredentials `
        -TimeoutSec 30 `
        -Headers @{ Accept = "application/json" } `
        -Uri $AdoApiUrl
    $payload = [string]$response.Content
    try {
        $item = $payload | ConvertFrom-Json
        if ($null -ne $item.content) {
            $payload = [string]$item.content
        }
    }
    catch {
        $payload = [string]$response.Content
    }
    return $payload
}

$health = Invoke-RestMethod -Uri "$baseUri/health" -TimeoutSec 10
if ($health.status -ne "healthy" -or $health.database.integrityStatus -ne "ok") {
    throw "Receiver registry is not healthy. Import was cancelled."
}

$sourceRoot = (Get-SourceJson) | ConvertFrom-Json
$sourceReceivers = @(Get-ReceiverNodes -Node $sourceRoot) |
    Group-Object IpAddress |
    ForEach-Object { $_.Group[0] } |
    Sort-Object { [version]$_.IpAddress }
if ($sourceReceivers.Count -eq 0) {
    throw "No valid receiver entries were found in the source configuration."
}

$sourceGroupNames = @{
    1 = "Beta"
    2 = "Alpha"
    3 = "Group 3"
    4 = "Group 4"
}

$existing = Invoke-RestMethod -Uri "$baseUri/api/v1/receivers?includeDisabled=true"
$existingByIp = @{}
foreach ($receiver in $existing) {
    $existingByIp[[string]$receiver.ipAddress] = $receiver
}

$plan = foreach ($source in $sourceReceivers) {
    $current = $existingByIp[$source.IpAddress]
    $action = if (-not $current) {
        "Create"
    }
    elseif (-not $current.enabled) {
        "Update"
    }
    else {
        "No change"
    }

    [pscustomobject]@{
        IpAddress = $source.IpAddress
        SourceGroup = $sourceGroupNames[$source.GroupId]
        Action = $action
    }
}

$plan | Format-Table -AutoSize
$changes = @($plan | Where-Object Action -ne "No change")
Write-Host "Source receivers: $($sourceReceivers.Count)"
Write-Host "Planned changes: $($changes.Count)"

if (-not $Apply) {
    Write-Host "Dry run completed. Run this script again with -Apply to write the changes."
    return
}

$backup = Invoke-RestMethod `
    -Method Post `
    -Uri "$baseUri/api/v1/maintenance/backup" `
    -Headers $headers
Write-Host "Verified backup created: $($backup.fullPath)"

foreach ($source in $sourceReceivers) {
    $current = $existingByIp[$source.IpAddress]
    if (-not $current) {
        $body = @{
            ipAddress = $source.IpAddress
            name = $null
            model = $null
            serialNumber = $null
            location = $null
            notes = "Imported from qa-receivers.json"
            enabled = $true
            groupIds = @()
            primaryGroupId = $null
        } | ConvertTo-Json -Depth 8
        Invoke-RestMethod `
            -Method Post `
            -Uri "$baseUri/api/v1/receivers" `
            -Headers $headers `
            -ContentType "application/json" `
            -Body $body | Out-Null
        continue
    }

    if ($current.enabled) {
        continue
    }

    $membershipIds = @($current.groups | ForEach-Object { [int]$_.id })
    $primaryGroup = $current.groups | Where-Object isPrimary | Select-Object -First 1
    $primaryGroupId = if ($primaryGroup) { [int]$primaryGroup.id } else { $null }
    $body = @{
        ipAddress = $current.ipAddress
        name = $current.name
        model = $current.model
        serialNumber = $current.serialNumber
        location = $current.location
        notes = $current.notes
        enabled = $true
        groupIds = @($membershipIds)
        primaryGroupId = $primaryGroupId
        rowVersion = $current.rowVersion
    } | ConvertTo-Json -Depth 8
    Invoke-RestMethod `
        -Method Put `
        -Uri "$baseUri/api/v1/receivers/$($current.id)" `
        -Headers $headers `
        -ContentType "application/json" `
        -Body $body | Out-Null
}

$finalReceivers = Invoke-RestMethod -Uri "$baseUri/api/v1/receivers"
Write-Host "Import completed successfully. Active registry receivers: $($finalReceivers.Count)"
