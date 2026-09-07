# WordPress Archive Viewer — Runtime Setup Script
# Downloads and configures portable PHP, MariaDB, and Caddy for development

param(
    [string]$RuntimeDir = (Join-Path $PSScriptRoot '..\runtime'),
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

# Versions
$PHP_VERSION = '8.3.33'
$MARIADB_VERSION = '11.4.5'
$CADDY_VERSION = '2.9.1'

# URLs
$PHP_URL = "https://windows.php.net/downloads/releases/php-$PHP_VERSION-nts-Win32-vs16-x64.zip"
$MARIADB_URL = "https://archive.mariadb.org/mariadb-$MARIADB_VERSION/winx64-packages/mariadb-$MARIADB_VERSION-winx64.zip"
$CADDY_URL = "https://github.com/caddyserver/caddy/releases/download/v$CADDY_VERSION/caddy_${CADDY_VERSION}_windows_amd64.zip"

$resolvedPath = Resolve-Path -Path $RuntimeDir -ErrorAction SilentlyContinue
if ($resolvedPath) {
    $RuntimeDir = $resolvedPath.Path
} else {
    $RuntimeDir = (New-Item -Path $RuntimeDir -ItemType Directory -Force).FullName
}

function Download-AndExtract {
    param(
        [string]$Url,
        [string]$Name,
        [string]$DestDir
    )

    $zipFile = Join-Path $env:TEMP "$Name.zip"

    if ((Test-Path $DestDir) -and -not $Force) {
        Write-Host "  [SKIP] $Name already exists at $DestDir" -ForegroundColor Yellow
        return
    }

    Write-Host "  [DOWNLOAD] $Name from $Url..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri $Url -OutFile $zipFile -UseBasicParsing

    Write-Host "  [EXTRACT] $Name to $DestDir..." -ForegroundColor Cyan
    if (Test-Path $DestDir) { Remove-Item $DestDir -Recurse -Force }
    New-Item -Path $DestDir -ItemType Directory -Force | Out-Null

    Expand-Archive -Path $zipFile -DestinationPath $DestDir -Force

    # Some archives have a single root folder — flatten it
    $children = Get-ChildItem -Path $DestDir
    if ($children.Count -eq 1 -and $children[0].PSIsContainer) {
        $innerDir = $children[0].FullName
        Get-ChildItem -Path $innerDir | Move-Item -Destination $DestDir -Force
        Remove-Item $innerDir -Force
    }

    Remove-Item $zipFile -Force -ErrorAction SilentlyContinue
    Write-Host "  [DONE] $Name ready" -ForegroundColor Green
}

function Setup-PhpIni {
    param([string]$PhpDir)

    $iniPath = Join-Path $PhpDir 'php.ini'
    $devIni = Join-Path $PhpDir 'php.ini-development'

    if (Test-Path $devIni) {
        Copy-Item $devIni $iniPath -Force
    } else {
        # Create minimal php.ini
        '' | Out-File $iniPath -Encoding UTF8
    }

    # Enable required extensions for WordPress
    $extensions = @(
        'curl', 'fileinfo', 'gd', 'intl', 'mbstring',
        'exif', 'mysqli', 'openssl', 'pdo_mysql',
        'zip', 'sodium'
    )

    $content = Get-Content $iniPath -Raw

    foreach ($ext in $extensions) {
        $content = $content -replace ('(?m)^;(extension=' + $ext + ')'), ('extension=' + $ext)
    }

    $content = $content -replace '(?m)^;?extension_dir\s*=.*$', 'extension_dir = "ext"'

    $settings = @(
        '',
        '; WordPress Archive Viewer settings',
        'max_execution_time = 300',
        'memory_limit = 256M',
        'post_max_size = 64M',
        'upload_max_filesize = 64M',
        'max_input_vars = 3000',
        'display_errors = Off',
        'log_errors = On',
        'error_log = php_errors.log'
    )
    $content += ($settings -join [Environment]::NewLine)

    $content | Out-File $iniPath -Encoding UTF8 -Force
    Write-Host '  [DONE] php.ini configured' -ForegroundColor Green
}

Write-Host ''
Write-Host '=== WordPress Archive Viewer — Runtime Setup ===' -ForegroundColor Magenta
Write-Host ''

# PHP
Write-Host "[1/3] Setting up PHP $PHP_VERSION..." -ForegroundColor White
$phpDir = Join-Path $RuntimeDir 'php'
Download-AndExtract -Url $PHP_URL -Name 'php' -DestDir $phpDir
Setup-PhpIni -PhpDir $phpDir

# MariaDB
Write-Host "[2/3] Setting up MariaDB $MARIADB_VERSION..." -ForegroundColor White
$mariaDir = Join-Path $RuntimeDir 'mariadb'
Download-AndExtract -Url $MARIADB_URL -Name 'mariadb' -DestDir $mariaDir

# Caddy
Write-Host "[3/3] Setting up Caddy $CADDY_VERSION..." -ForegroundColor White
$caddyDir = Join-Path $RuntimeDir 'caddy'
Download-AndExtract -Url $CADDY_URL -Name 'caddy' -DestDir $caddyDir

Write-Host ''
Write-Host '=== Runtime setup complete! ===' -ForegroundColor Green
Write-Host ''
Write-Host ('Runtime directory: ' + $RuntimeDir) -ForegroundColor Gray
$phpExe = Join-Path $phpDir 'php-cgi.exe'
$mariaExe = Join-Path $mariaDir 'bin\mariadbd.exe'
$caddyExe = Join-Path $caddyDir 'caddy.exe'
Write-Host 'Runtime binaries downloaded.' -ForegroundColor Gray
Write-Host ''
