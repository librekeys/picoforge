Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$env:LIB = $null
$env:LIBPATH = $null
Add-Type -MemberDefinition '[DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, int e); [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);' -Name Win -Namespace Native

$exe = "C:\Users\guoxin\Desktop\桌面文件夹\项目\picoforge\target\debug\picoforge.exe"
$out = "C:\Users\guoxin\Desktop\桌面文件夹\项目\picoforge\temp\i18n\screen_menu.png"

$p = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 9
[Native.Win]::SetForegroundWindow($p.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 500

[System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point(330, 893)
Start-Sleep -Milliseconds 400
[Native.Win]::mouse_event(0x0002, 0, 0, 0, 0)
Start-Sleep -Milliseconds 60
[Native.Win]::mouse_event(0x0004, 0, 0, 0, 0)
Start-Sleep -Seconds 2

$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Stop-Process -Id $p.Id -Force
Write-Output "saved $out"
