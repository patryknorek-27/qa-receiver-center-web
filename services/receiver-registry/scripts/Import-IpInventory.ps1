[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string]$WorkbookPath,

    [string]$RegistryBaseUri = "http://HHKNQA1-DE-DE04.eu.trimblecorp.net:8787",

    [string]$Actor = "",

    [string]$ExportSeedMigrationPath = "",

    [switch]$SourceOnly,

    [switch]$Apply
)

$ErrorActionPreference = "Stop"
$WorkbookPath = [System.IO.Path]::GetFullPath($WorkbookPath)
$RegistryBaseUri = $RegistryBaseUri.TrimEnd("/")
if ([string]::IsNullOrWhiteSpace($Actor)) {
    $Actor = if ($env:USERDOMAIN) { "$($env:USERDOMAIN)\$($env:USERNAME)" } else { $env:USERNAME }
}

function Read-ZipEntryText {
    param(
        [System.IO.Compression.ZipArchive]$Archive,
        [string]$EntryName
    )

    $entry = $Archive.GetEntry($EntryName)
    if (-not $entry) {
        throw "The workbook entry was not found: $EntryName"
    }

    $stream = $entry.Open()
    $reader = [System.IO.StreamReader]::new($stream)
    try {
        return $reader.ReadToEnd()
    }
    finally {
        $reader.Dispose()
        $stream.Dispose()
    }
}

function Get-XlsxSharedStrings {
    param([System.IO.Compression.ZipArchive]$Archive)

    if (-not $Archive.GetEntry("xl/sharedStrings.xml")) {
        return @()
    }

    [xml]$xml = Read-ZipEntryText -Archive $Archive -EntryName "xl/sharedStrings.xml"
    $namespace = [System.Xml.XmlNamespaceManager]::new($xml.NameTable)
    $namespace.AddNamespace("d", "http://schemas.openxmlformats.org/spreadsheetml/2006/main")
    return @($xml.SelectNodes("//d:si", $namespace) | ForEach-Object {
        ($_.SelectNodes(".//d:t", $namespace) | ForEach-Object { $_.InnerText }) -join ""
    })
}

function Get-XlsxSheetRows {
    param(
        [System.IO.Compression.ZipArchive]$Archive,
        [string[]]$SharedStrings,
        [string]$SheetName
    )

    [xml]$workbook = Read-ZipEntryText -Archive $Archive -EntryName "xl/workbook.xml"
    [xml]$relationships = Read-ZipEntryText -Archive $Archive -EntryName "xl/_rels/workbook.xml.rels"
    $workbookNamespace = [System.Xml.XmlNamespaceManager]::new($workbook.NameTable)
    $workbookNamespace.AddNamespace("d", "http://schemas.openxmlformats.org/spreadsheetml/2006/main")
    $relationshipNamespace = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
    $sheet = $workbook.SelectNodes("//d:sheet", $workbookNamespace) |
        Where-Object { $_.GetAttribute("name") -eq $SheetName } |
        Select-Object -First 1
    if (-not $sheet) {
        throw "The workbook does not contain sheet '$SheetName'."
    }

    $relationshipId = $sheet.GetAttribute("id", $relationshipNamespace)
    $packageNamespace = [System.Xml.XmlNamespaceManager]::new($relationships.NameTable)
    $packageNamespace.AddNamespace("r", "http://schemas.openxmlformats.org/package/2006/relationships")
    $relationship = $relationships.SelectNodes("//r:Relationship", $packageNamespace) |
        Where-Object { $_.GetAttribute("Id") -eq $relationshipId } |
        Select-Object -First 1
    if (-not $relationship) {
        throw "The worksheet relationship for '$SheetName' was not found."
    }

    $target = $relationship.GetAttribute("Target").Replace("\", "/")
    $entryName = if ($target.StartsWith("/")) {
        $target.TrimStart("/")
    }
    elseif ($target.StartsWith("xl/")) {
        $target
    }
    else {
        "xl/$target"
    }

    [xml]$worksheet = Read-ZipEntryText -Archive $Archive -EntryName $entryName
    $sheetNamespace = [System.Xml.XmlNamespaceManager]::new($worksheet.NameTable)
    $sheetNamespace.AddNamespace("d", "http://schemas.openxmlformats.org/spreadsheetml/2006/main")
    return @($worksheet.SelectNodes("//d:sheetData/d:row", $sheetNamespace) | ForEach-Object {
        $values = @{}
        foreach ($cell in $_.SelectNodes("d:c", $sheetNamespace)) {
            $reference = $cell.GetAttribute("r")
            $column = ([regex]::Match($reference, "^[A-Z]+")).Value
            $type = $cell.GetAttribute("t")
            if ($type -eq "inlineStr") {
                $value = ($cell.SelectNodes(".//d:t", $sheetNamespace) | ForEach-Object { $_.InnerText }) -join ""
            }
            else {
                $valueNode = $cell.SelectSingleNode("d:v", $sheetNamespace)
                $value = if ($valueNode) { $valueNode.InnerText } else { $null }
                if ($type -eq "s" -and $null -ne $value) {
                    $value = $SharedStrings[[int]$value]
                }
            }
            $values[$column] = $value
        }

        [pscustomobject]@{
            RowNumber = [int]$_.GetAttribute("r")
            Values = $values
        }
    })
}

function Normalize-Text {
    param([object]$Value)
    $text = if ($null -eq $Value) { "" } else { [string]$Value }
    $text = $text.Trim()
    return $(if ($text.Length -eq 0) { $null } else { $text })
}

function ConvertTo-SqlLiteral {
    param([object]$Value)

    if ($null -eq $Value -or [string]::IsNullOrWhiteSpace([string]$Value) -or [string]$Value -eq "?") {
        return "NULL"
    }
    return "'$(([string]$Value).Replace("'", "''"))'"
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [System.IO.Compression.ZipFile]::OpenRead($WorkbookPath)
try {
    $sharedStrings = @(Get-XlsxSharedStrings -Archive $archive)
    $mainRows = @(Get-XlsxSheetRows -Archive $archive -SharedStrings $sharedStrings -SheetName "Sheet1")
    $secondaryRows = @(Get-XlsxSheetRows -Archive $archive -SharedStrings $sharedStrings -SheetName "Sheet4")
}
finally {
    $archive.Dispose()
}

$errors = [System.Collections.Generic.List[string]]::new()
$warnings = [System.Collections.Generic.List[string]]::new()
$sourceByIp = @{}
foreach ($row in $mainRows | Where-Object RowNumber -ge 3) {
    $ipAddress = Normalize-Text $row.Values["M"]
    if (-not $ipAddress) {
        continue
    }

    $match = [regex]::Match($ipAddress, "^10\.2\.155\.(\d{1,3})$")
    if (-not $match.Success -or [int]$match.Groups[1].Value -lt 2 -or [int]$match.Groups[1].Value -gt 254) {
        $errors.Add("Sheet1 row $($row.RowNumber) contains an invalid IP address: $ipAddress")
        continue
    }
    if ($sourceByIp.ContainsKey($ipAddress)) {
        $errors.Add("Sheet1 contains the IP address more than once: $ipAddress")
        continue
    }

    $rawStatus = Normalize-Text $row.Values["R"]
    $status = switch -Regex ($rawStatus) {
        "^(?i:used)$" { "Used"; break }
        "^(?i:unused)$" { "Unused"; break }
        "^(?i:reserved)$" { "Reserved"; break }
        default { $null }
    }
    if (-not $status) {
        $errors.Add("Sheet1 row $($row.RowNumber) has an invalid status for ${ipAddress}: '$rawStatus'")
        continue
    }

    $record = [pscustomobject]@{
        IpAddress = $ipAddress
        SortOrder = [int]$match.Groups[1].Value
        Status = $status
        Responsible = Normalize-Text $row.Values["N"]
        DeviceType = Normalize-Text $row.Values["O"]
        SerialNumber = Normalize-Text $row.Values["P"]
        Location = Normalize-Text $row.Values["Q"]
        SourceRow = $row.RowNumber
    }
    $sourceByIp[$ipAddress] = $record
    if ($status -eq "Unused" -and ($record.Responsible -or $record.DeviceType -or $record.SerialNumber)) {
        $warnings.Add("$ipAddress is Unused but still contains device information in Sheet1.")
    }
    if ($record.Location -eq "?") {
        $warnings.Add("$ipAddress has an unknown location marked as '?'.")
    }
}

foreach ($octet in 2..254) {
    $ipAddress = "10.2.155.$octet"
    if (-not $sourceByIp.ContainsKey($ipAddress)) {
        $errors.Add("Sheet1 does not contain $ipAddress.")
    }
}

foreach ($row in $secondaryRows) {
    $ipAddress = Normalize-Text $row.Values["A"]
    if (-not $ipAddress -or -not $sourceByIp.ContainsKey($ipAddress)) {
        continue
    }
    $main = $sourceByIp[$ipAddress]
    $comparisons = @(
        @{ Name = "responsible"; Main = $main.Responsible; Secondary = (Normalize-Text $row.Values["B"]) },
        @{ Name = "device"; Main = $main.DeviceType; Secondary = (Normalize-Text $row.Values["C"]) },
        @{ Name = "misc/SN"; Main = $main.SerialNumber; Secondary = (Normalize-Text $row.Values["D"]) }
    )
    foreach ($comparison in $comparisons) {
        if ($comparison.Secondary -and $comparison.Main -ne $comparison.Secondary) {
            $warnings.Add("$ipAddress has different $($comparison.Name) data in Sheet1 and Sheet4.")
        }
    }
}

$sourceRecords = @($sourceByIp.Values | Sort-Object SortOrder)
$usedCount = @($sourceRecords | Where-Object Status -eq "Used").Count
$unusedCount = @($sourceRecords | Where-Object Status -eq "Unused").Count
$reservedCount = @($sourceRecords | Where-Object Status -eq "Reserved").Count
Write-Host "Workbook: $WorkbookPath"
Write-Host "Source records: $($sourceRecords.Count)"
Write-Host "Used: $usedCount; Unused: $unusedCount; Reserved: $reservedCount"
Write-Host "Warnings: $($warnings.Count); Errors: $($errors.Count)"

if ($warnings.Count -gt 0) {
    Write-Host ""
    Write-Host "Warnings:"
    $warnings | ForEach-Object { Write-Host "  $_" }
}
if ($errors.Count -gt 0) {
    Write-Host ""
    Write-Host "Errors:"
    $errors | ForEach-Object { Write-Host "  $_" }
    throw "Workbook validation failed. Nothing was written."
}
if (-not [string]::IsNullOrWhiteSpace($ExportSeedMigrationPath)) {
    if ($Apply) {
        throw "ExportSeedMigrationPath and Apply cannot be used together."
    }

    $resolvedMigrationPath = [System.IO.Path]::GetFullPath($ExportSeedMigrationPath)
    $migrationLines = [System.Collections.Generic.List[string]]::new()
    $migrationLines.Add("CREATE TEMP TABLE ip_inventory_seed_guard (")
    $migrationLines.Add("    pristine_record_count INTEGER NOT NULL CHECK (pristine_record_count = 253)")
    $migrationLines.Add(");")
    $migrationLines.Add("")
    $migrationLines.Add("INSERT INTO ip_inventory_seed_guard (pristine_record_count)")
    $migrationLines.Add("SELECT COUNT(*)")
    $migrationLines.Add("FROM ip_inventory")
    $migrationLines.Add("WHERE sort_order BETWEEN 2 AND 254")
    $migrationLines.Add("  AND row_version = 1")
    $migrationLines.Add("  AND updated_by = 'database-migration';")
    $migrationLines.Add("")

    foreach ($record in $sourceRecords) {
        $migrationLines.Add("UPDATE ip_inventory")
        $migrationLines.Add("SET status = $(ConvertTo-SqlLiteral $record.Status),")
        $migrationLines.Add("    location = $(ConvertTo-SqlLiteral $record.Location),")
        $migrationLines.Add("    responsible = $(ConvertTo-SqlLiteral $record.Responsible),")
        $migrationLines.Add("    device_type = $(ConvertTo-SqlLiteral $record.DeviceType),")
        $migrationLines.Add("    serial_number = $(ConvertTo-SqlLiteral $record.SerialNumber),")
        $migrationLines.Add("    row_version = row_version + 1,")
        $migrationLines.Add("    updated_by = 'static-ip-overview-migration',")
        $migrationLines.Add("    updated_at = CURRENT_TIMESTAMP")
        $migrationLines.Add("WHERE ip_address = $(ConvertTo-SqlLiteral $record.IpAddress);")
        $migrationLines.Add("")
    }

    $migrationLines.Add("INSERT INTO audit_log")
    $migrationLines.Add("    (entity_type, entity_id, action, actor, changed_at, payload_json)")
    $migrationLines.Add("VALUES")
    $migrationLines.Add("    ('ip-inventory', '10.2.155.0/24', 'seeded', 'database-migration', CURRENT_TIMESTAMP,")
    $migrationLines.Add("     '{`"source`":`"Static IP Overview.xlsx`",`"records`":253,`"used`":$usedCount,`"unused`":$unusedCount}');")
    $migrationLines.Add("")
    $migrationLines.Add("DROP TABLE ip_inventory_seed_guard;")

    $migrationDirectory = Split-Path -Parent $resolvedMigrationPath
    if (-not [string]::IsNullOrWhiteSpace($migrationDirectory)) {
        New-Item -ItemType Directory -Path $migrationDirectory -Force | Out-Null
    }
    [IO.File]::WriteAllLines(
        $resolvedMigrationPath,
        $migrationLines,
        (New-Object Text.UTF8Encoding($false)))
    Write-Host "Seed migration created: $resolvedMigrationPath"
    Write-Host "Nothing was written to QA Receiver Registry."
    return
}
if ($SourceOnly) {
    if ($Apply) {
        throw "SourceOnly and Apply cannot be used together."
    }
    Write-Host "Source validation completed. Nothing was written."
    return
}
$headers = @{
    "X-QA-User" = $Actor
}
$currentRecords = Invoke-RestMethod -Uri "$RegistryBaseUri/api/v1/ip-addresses" -Headers $headers
if ($currentRecords.Count -ne 254) {
    throw "QA Receiver Registry returned $($currentRecords.Count) IP records instead of 254."
}
$currentByIp = @{}
$currentRecords | ForEach-Object { $currentByIp[$_.ipAddress] = $_ }
$plan = foreach ($source in $sourceRecords) {
    $current = $currentByIp[$source.IpAddress]
    if (-not $current) {
        throw "QA Receiver Registry does not contain $($source.IpAddress)."
    }
    $changed = $current.status -ne $source.Status -or
        $current.location -ne $source.Location -or
        $current.responsible -ne $source.Responsible -or
        $current.deviceType -ne $source.DeviceType -or
        $current.serialNumber -ne $source.SerialNumber
    $manualConflict = $changed -and ($current.rowVersion -gt 1 -or $current.updatedBy -ne "database-migration")
    [pscustomobject]@{
        IpAddress = $source.IpAddress
        CurrentStatus = $current.status
        SourceStatus = $source.Status
        Action = if ($manualConflict) { "Review" } elseif ($changed) { "Update" } else { "No change" }
        Source = $source
        Current = $current
    }
}

$plan | Select-Object IpAddress, CurrentStatus, SourceStatus, Action | Format-Table -AutoSize
$changes = @($plan | Where-Object Action -eq "Update")
$conflicts = @($plan | Where-Object Action -eq "Review")
Write-Host "Planned updates: $($changes.Count)"
Write-Host "Manual review required: $($conflicts.Count)"
if (-not $Apply) {
    Write-Host "Dry run completed. Run again with -Apply only after reviewing the results."
    return
}
if ($conflicts.Count -gt 0) {
    throw "Import cancelled because $($conflicts.Count) records require manual review. Nothing was written."
}
if ($changes.Count -eq 0) {
    Write-Host "No changes are required. Nothing was written."
    return
}

$items = @($plan | ForEach-Object {
    @{
        ipAddress = $_.Source.IpAddress
        status = $_.Source.Status
        location = $_.Source.Location
        responsible = $_.Source.Responsible
        environment = $_.Current.environment
        deviceType = $_.Source.DeviceType
        serialNumber = $_.Source.SerialNumber
        antenna1 = $_.Current.antenna1
        antenna2 = $_.Current.antenna2
        notes = $_.Current.notes
        usageEndDate = $_.Current.usageEndDate
        rowVersion = $_.Current.rowVersion
    }
})
$body = @{ items = $items } | ConvertTo-Json -Depth 8
$bodyBytes = [Text.Encoding]::UTF8.GetBytes($body)
$result = Invoke-RestMethod `
    -Method Put `
    -Uri "$RegistryBaseUri/api/v1/ip-addresses/import" `
    -Headers $headers `
    -ContentType "application/json" `
    -Body $bodyBytes
if ($result.updatedCount -ne 253 -or -not $result.backup -or $result.backup.sizeBytes -le 0) {
    throw "The server did not confirm a complete import and verified backup."
}

Write-Host "Import completed successfully. Updated records: $($result.updatedCount)"
Write-Host "Verified database backup: $($result.backup.fullPath)"
