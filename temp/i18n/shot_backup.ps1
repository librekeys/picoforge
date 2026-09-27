Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class M {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, int e);
  public static void Click(int x, int y){ SetCursorPos(x,y); mouse_event(0x02,0,0,0,0); System.Threading.Thread.Sleep(80); mouse_event(0x04,0,0,0,0); }
}
"@

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$exe = Join-Path $root 'target\debug\picoforge.exe'
$out = Join-Path $PSScriptRoot 'screen_backup_zh.png'

$p = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 9
[M]::Click(207, 551)
Start-Sleep -Seconds 3

$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Stop-Process -Id $p.Id -Force
Write-Output "saved $out"
