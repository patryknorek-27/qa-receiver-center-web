$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$mapPath = Join-Path $repositoryRoot 'docs\MAPA-KODU.md'
$handoffPath = Join-Path $repositoryRoot 'docs\AI-SESSION-HANDOFF.md'
$packagePath = Join-Path $repositoryRoot 'package.json'

function Resolve-RepositoryPath([string] $relativePath) {
    return Join-Path $repositoryRoot ($relativePath -replace '/', '\')
}

function Assert-ExistingPath([string] $relativePath, [string] $sourcePath) {
    if (-not (Test-Path -LiteralPath (Resolve-RepositoryPath $relativePath))) {
        throw "Missing documented path '$relativePath' referenced by $sourcePath."
    }
}

foreach ($document in @($mapPath, $handoffPath)) {
    if (-not (Test-Path -LiteralPath $document)) {
        throw "Missing repository document '$document'."
    }
}

$mapContent = Get-Content -LiteralPath $mapPath -Raw
$handoffContent = Get-Content -LiteralPath $handoffPath -Raw

foreach ($match in [regex]::Matches($mapContent, '<!-- check-path: (?<path>[^ ]+) -->')) {
    Assert-ExistingPath $match.Groups['path'].Value 'docs/MAPA-KODU.md'
}

foreach ($match in [regex]::Matches($handoffContent, '<!-- check-path: (?<path>[^ ]+) -->')) {
    Assert-ExistingPath $match.Groups['path'].Value 'docs/AI-SESSION-HANDOFF.md'
}

foreach ($match in [regex]::Matches($handoffContent, '<!-- check-doc: (?<path>[^ ]+) -->')) {
    Assert-ExistingPath $match.Groups['path'].Value 'docs/AI-SESSION-HANDOFF.md'
}

$package = Get-Content -LiteralPath $packagePath -Raw | ConvertFrom-Json
foreach ($match in [regex]::Matches($mapContent, '<!-- check-script: (?<script>[^ ]+) -->')) {
    $scriptName = $match.Groups['script'].Value
    if ($null -eq $package.scripts.$scriptName) {
        throw "Missing npm script '$scriptName' referenced by docs/MAPA-KODU.md."
    }
}

Write-Output 'Repository documentation checks passed.'
