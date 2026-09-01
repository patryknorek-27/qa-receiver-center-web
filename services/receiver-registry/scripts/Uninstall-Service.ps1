[CmdletBinding(SupportsShouldProcess)]
param()

$ErrorActionPreference = "Stop"
$serviceName = "QAReceiverRegistry"
$firewallRuleName = "QA Receiver Registry HTTPS"

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "Run this script from an elevated PowerShell window."
}

$service = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
if ($service -and $PSCmdlet.ShouldProcess($serviceName, "Stop and remove Windows service")) {
    if ($service.Status -ne "Stopped") {
        Stop-Service -Name $serviceName -Force
        $service.WaitForStatus("Stopped", [TimeSpan]::FromSeconds(30))
    }

    & sc.exe delete $serviceName | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "Could not remove service $serviceName."
    }
}

$firewallRule = Get-NetFirewallRule -DisplayName $firewallRuleName -ErrorAction SilentlyContinue
if ($firewallRule -and $PSCmdlet.ShouldProcess($firewallRuleName, "Remove firewall rule")) {
    $firewallRule | Remove-NetFirewallRule
}

Write-Host "Service registration and firewall rule were removed."
Write-Host "Database, configuration, clone files, backups, certificate, and application files were preserved."
