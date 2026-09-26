<#
  Allow normal (non-elevated) user processes to open the Windows FIDO HID
  interface, so tools using hidapi/CTAPHID (e.g. picoforge) can detect the key.

  Root cause: Windows binds the FIDO HID collection to the in-box `fidohid`
  driver, whose devnode security descriptor grants only
  Administrators / SYSTEM / NT SERVICE\CryptSvc. hidapi's hid_enumerate throws
  away any interface it cannot CreateFile, so the FIDO interface disappears and
  the app reports "No FIDO device found".

  This script appends an ACE granting Builtin Users GENERIC_ALL to every present
  FIDO HID devnode (hardware id HID_DEVICE_UP:F1D0_U:0001), then restarts the
  device so the live interface SD is regenerated.

  Run elevated. Requires the key plugged in.
#>
$ErrorActionPreference = 'Stop'

$fidoNodes = Get-PnpDevice -PresentOnly -Class HIDClass |
    Where-Object { $_.InstanceId -like 'HID\*' -and $_.HardwareID -contains 'HID_DEVICE_UP:F1D0_U:0001' }

if (-not $fidoNodes) { Write-Host 'No present FIDO HID device found.'; exit 1 }

foreach ($node in $fidoNodes) {
    $key = "HKLM:\SYSTEM\CurrentControlSet\Enum\$($node.InstanceId)"
    Write-Host "Devnode: $($node.InstanceId)"
    if (-not (Test-Path $key)) { Write-Host '  (no Enum key, skipped)'; continue }

    $bytes = (Get-ItemProperty -Path $key -Name Security).Security
    $sd = New-Object System.Security.AccessControl.RawSecurityDescriptor -ArgumentList $bytes, 0
    $sddl = $sd.GetSddlForm([System.Security.AccessControl.AccessControlSections]::All)
    Write-Host "  before: $sddl"

    if ($sddl -match '\(A;;GA;;;BU\)') {
        Write-Host '  Builtin Users already allowed.'
    } else {
        $newSd = New-Object System.Security.AccessControl.RawSecurityDescriptor($sddl + '(A;;GA;;;BU)')
        $newBytes = New-Object byte[] $newSd.BinaryLength
        $newSd.GetBinaryForm($newBytes, 0)
        Set-ItemProperty -Path $key -Name Security -Value $newBytes -Type Binary
        Write-Host "  after : $($newSd.GetSddlForm([System.Security.AccessControl.AccessControlSections]::All))"
    }

    Write-Host '  restarting device...'
    pnputil /restart-device $node.InstanceId | Out-Host
}
Write-Host 'Done. Replug the key if the app still does not see FIDO.'
