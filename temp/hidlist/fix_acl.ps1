$ErrorActionPreference='Continue'
$log = Join-Path $PSScriptRoot 'fix_acl.out'
"start" | Out-File $log
$k='HKLM:\SYSTEM\CurrentControlSet\Enum\HID\VID_1050&PID_0407&MI_01\7&3a1fed9c&0&0000'
try {
  $b=(Get-ItemProperty $k).Security
  $sd=New-Object System.Security.AccessControl.RawSecurityDescriptor -ArgumentList $b,0
  $sddl=$sd.GetSddlForm([System.Security.AccessControl.AccessControlSections]::All)
  "before: $sddl" | Out-File $log -Append
  $new = $sddl + '(A;;GA;;;BU)'
  $nsd=New-Object System.Security.AccessControl.RawSecurityDescriptor($new)
  $bytes=New-Object byte[] $nsd.BinaryLength
  $nsd.GetBinaryForm($bytes,0)
  Set-ItemProperty -Path $k -Name Security -Value $bytes -Type Binary -ErrorAction Stop
  "written" | Out-File $log -Append
} catch { "ERR: $_" | Out-File $log -Append }
$b2=(Get-ItemProperty $k).Security
$sd2=New-Object System.Security.AccessControl.RawSecurityDescriptor -ArgumentList $b2,0
"after: $($sd2.GetSddlForm([System.Security.AccessControl.AccessControlSections]::All))" | Out-File $log -Append
"restart:" | Out-File $log -Append
(pnputil /restart-device "HID\VID_1050&PID_0407&MI_01\7&3a1fed9c&0&0000" 2>&1 | Out-String) | Out-File $log -Append
