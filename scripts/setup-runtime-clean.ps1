$ErrorActionPreference = "Stop"

$RuntimeDir = Join-Path $PWD "runtime"
if (-not (Test-Path $RuntimeDir)) { New-Item -Path $RuntimeDir -ItemType Directory -Force | Out-Null }

function Download-AndExtract {
    param([string]$Url, [string]$Name, [string]$DestDir)
    if (Test-Path $DestDir) { return }
    $zipFile = Join-Path $env:TEMP "$Name.zip"
    Write-Host "Downloading $Name from $Url..."
    Invoke-WebRequest -Uri $Url -OutFile $zipFile -UseBasicParsing
    Write-Host "Extracting $Name..."
    Expand-Archive -Path $zipFile -DestinationPath $DestDir -Force
    $children = Get-ChildItem -Path $DestDir
    if ($children.Count -eq 1 -and $children[0].PSIsContainer) {
        $innerDir = $children[0].FullName
        Get-ChildItem -Path $innerDir | Move-Item -Destination $DestDir -Force
        Remove-Item $innerDir -Force
    }
    Remove-Item $zipFile -Force -ErrorAction SilentlyContinue
}

$PHP_URL = "https://windows.php.net/downloads/releases/archives/php-8.3.8-nts-Win32-vs16-x64.zip"
$MARIADB_URL = "https://archive.mariadb.org/mariadb-11.4.2/winx64-packages/mariadb-11.4.2-winx64.zip"
$CADDY_URL = "https://github.com/caddyserver/caddy/releases/download/v2.7.6/caddy_2.7.6_windows_amd64.zip"

Download-AndExtract -Url $PHP_URL -Name "php" -DestDir (Join-Path $RuntimeDir "php")
Download-AndExtract -Url $MARIADB_URL -Name "mariadb" -DestDir (Join-Path $RuntimeDir "mariadb")
Download-AndExtract -Url $CADDY_URL -Name "caddy" -DestDir (Join-Path $RuntimeDir "caddy")

$PhpDir = Join-Path $RuntimeDir "php"
$iniPath = Join-Path $PhpDir "php.ini"
if (-not (Test-Path $iniPath)) {
    Copy-Item (Join-Path $PhpDir "php.ini-development") $iniPath -Force
    $extDir = (Join-Path $PhpDir "ext") -replace '\\', '/'
    $content = Get-Content $iniPath -Raw
    $exts = @("curl","fileinfo","gd","intl","mbstring","exif","mysqli","openssl","pdo_mysql","zip","sodium")
    foreach ($ext in $exts) { $content = $content -replace "^;(extension=$ext)`$", "extension=$ext" }
    $content = $content -replace '^;?extension_dir\s*=.*$', "extension_dir = `"$extDir`""
    $content | Out-File $iniPath -Encoding UTF8 -Force
}
Write-Host "Setup Complete!"
