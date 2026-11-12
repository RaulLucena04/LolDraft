Add-Type -AssemblyName System.Drawing

$size = 1024
$bitmap = New-Object System.Drawing.Bitmap($size, $size)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit

# Fondo oscuro
$graphics.Clear([System.Drawing.Color]::FromArgb(255, 11, 15, 20))

# Mitad azul y mitad roja, separadas por una diagonal sobria
$blue = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 30, 58, 95))
$red = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 74, 29, 29))
$bluePoints = @(
    (New-Object System.Drawing.Point(0, 0)),
    (New-Object System.Drawing.Point(650, 0)),
    (New-Object System.Drawing.Point(374, 1024)),
    (New-Object System.Drawing.Point(0, 1024))
)
$redPoints = @(
    (New-Object System.Drawing.Point(650, 0)),
    (New-Object System.Drawing.Point(1024, 0)),
    (New-Object System.Drawing.Point(1024, 1024)),
    (New-Object System.Drawing.Point(374, 1024))
)
$graphics.FillPolygon($blue, $bluePoints)
$graphics.FillPolygon($red, $redPoints)

# Borde dorado
$gold = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(255, 212, 175, 55), 18)
$gold.Alignment = [System.Drawing.Drawing2D.PenAlignment]::Inset
$graphics.DrawRectangle($gold, 36, 36, 952, 952)

# Monograma LD
$font = New-Object System.Drawing.Font('Segoe UI', 300, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$format = New-Object System.Drawing.StringFormat
$format.Alignment = [System.Drawing.StringAlignment]::Center
$format.LineAlignment = [System.Drawing.StringAlignment]::Center
$textBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 230, 237, 245))
$graphics.DrawString('LD', $font, $textBrush, (New-Object System.Drawing.RectangleF(0, 20, 1024, 984)), $format)

$outDir = Join-Path $PSScriptRoot '..\apps\desktop\src-tauri-icon.png'
$full = [System.IO.Path]::GetFullPath($outDir)
$bitmap.Save($full, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()
Write-Output $full
