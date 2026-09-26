Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$exe = "C:\Users\guoxin\Desktop\桌面文件夹\项目\picoforge\target\debug\picoforge.exe"
$out = "C:\Users\guoxin\Desktop\桌面文件夹\项目\picoforge\temp\i18n\screen_ar.png"

$p = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 9

$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()

Stop-Process -Id $p.Id -Force
Write-Output "saved $out"
