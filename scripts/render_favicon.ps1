# Render the repository's two-shape vector mark as Minecraft's 64x64 PNG icon.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$vectaRoot = Split-Path -Parent $PSScriptRoot
[xml]$vectaSvg = Get-Content -LiteralPath (Join-Path $vectaRoot 'assets/vecta-icon.svg') -Raw
$vectaBitmap = [Drawing.Bitmap]::new(256, 256)
$vectaGraphics = [Drawing.Graphics]::FromImage($vectaBitmap)
$vectaBackground = [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml($vectaSvg.svg.rect.fill))
$vectaForeground = [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml($vectaSvg.svg.polygon.fill))
$vectaOutput = $null
try {
    $vectaGraphics.ScaleTransform(4, 4)
    $vectaGraphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $vectaGraphics.FillRectangle($vectaBackground, 0, 0, 64, 64)
    [Drawing.PointF[]]$vectaPoints = $vectaSvg.svg.polygon.points.Split(' ') | ForEach-Object {
        $vectaCoordinates = $_.Split(',')
        [Drawing.PointF]::new([float]$vectaCoordinates[0], [float]$vectaCoordinates[1])
    }
    $vectaGraphics.FillPolygon($vectaForeground, $vectaPoints)
    $vectaOutput = [Drawing.Bitmap]::new($vectaBitmap, [Drawing.Size]::new(64, 64))
    $vectaOutput.Save((Join-Path $vectaRoot 'package-content/favicon.png'), [Drawing.Imaging.ImageFormat]::Png)
} finally {
    if ($vectaOutput) { $vectaOutput.Dispose() }
    $vectaForeground.Dispose()
    $vectaBackground.Dispose()
    $vectaGraphics.Dispose()
    $vectaBitmap.Dispose()
}
