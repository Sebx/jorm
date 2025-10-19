# PowerShell script to install Jorm-RS on Windows
param(
    [string]$InstallPath = "$env:LOCALAPPDATA\jorm-rs",
    [switch]$Force,
    [switch]$SkipDependencies
)

# Colors for output
$Colors = @{
    Red = "Red"
    Green = "Green"
    Yellow = "Yellow"
    Blue = "Cyan"
    White = "White"
}

function Write-Status {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor $Colors.Blue
}

function Write-Success {
    param([string]$Message)
    Write-Host "[SUCCESS] $Message" -ForegroundColor $Colors.Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARNING] $Message" -ForegroundColor $Colors.Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor $Colors.Red
}

# Check if running as Administrator
if (([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")) {
    Write-Warning "Running as Administrator. This is not recommended for security reasons."
    if (-not $Force) {
        Write-Error "Please run this script as a regular user. Use -Force to override this check."
        exit 1
    }
}

Write-Host "🚀 Installing Jorm-RS on Windows..." -ForegroundColor $Colors.Green

# Detect architecture
$Arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }
Write-Status "Detected architecture: $Arch"

# Check for required dependencies
if (-not $SkipDependencies) {
    Write-Status "Checking dependencies..."
    
    # Check for PowerShell version
    if ($PSVersionTable.PSVersion.Major -lt 5) {
        Write-Error "PowerShell 5.0 or later is required"
        exit 1
    }
    
    # Check for .NET Framework
    $DotNetVersion = Get-ItemProperty "HKLM:SOFTWARE\Microsoft\NET Framework Setup\NDP\v4\Full\" -Name Release -ErrorAction SilentlyContinue
    if (-not $DotNetVersion -or $DotNetVersion.Release -lt 461808) {
        Write-Warning ".NET Framework 4.7.2 or later is recommended"
    }
    
    # Check for Python (optional but recommended)
    try {
        $PythonVersion = python --version 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Status "Python found: $PythonVersion"
        } else {
            Write-Warning "Python not found. Some features may not work properly."
            Write-Warning "Consider installing Python from: https://python.org"
        }
    } catch {
        Write-Warning "Python not found. Some features may not work properly."
    }
}

# Create installation directory
Write-Status "Creating installation directory: $InstallPath"
if (!(Test-Path $InstallPath)) {
    try {
        New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
    } catch {
        Write-Error "Failed to create installation directory: $_"
        exit 1
    }
}

# Download URL
$DownloadUrl = "https://github.com/${{ github.repository }}/releases/latest/download/jorm-rs-windows-${Arch}.zip"
$ZipPath = Join-Path $InstallPath "jorm-rs.zip"

Write-Status "Downloading Jorm-RS from: $DownloadUrl"

# Download
try {
    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing
    $ProgressPreference = 'Continue'
} catch {
    Write-Error "Failed to download Jorm-RS: $_"
    exit 1
}

# Verify download
if (!(Test-Path $ZipPath)) {
    Write-Error "Downloaded file not found"
    exit 1
}

# Extract
Write-Status "Extracting Jorm-RS..."
try {
    Expand-Archive -Path $ZipPath -DestinationPath $InstallPath -Force
    Remove-Item $ZipPath -Force
} catch {
    Write-Error "Failed to extract Jorm-RS: $_"
    exit 1
}

# Verify binary
$BinaryPath = Join-Path $InstallPath "jorm-rs.exe"
if (!(Test-Path $BinaryPath)) {
    Write-Error "Jorm-RS binary not found in archive"
    exit 1
}

# Test binary
Write-Status "Testing Jorm-RS binary..."
try {
    $Version = & $BinaryPath --version
    if ($LASTEXITCODE -ne 0) {
        throw "Binary test failed"
    }
    Write-Success "Jorm-RS binary test passed: $Version"
} catch {
    Write-Error "Jorm-RS binary test failed: $_"
    exit 1
}

# Add to PATH
Write-Status "Adding Jorm-RS to PATH..."
$CurrentPath = [Environment]::GetEnvironmentVariable("PATH", "User")

if ($CurrentPath -notlike "*$InstallPath*") {
    try {
        [Environment]::SetEnvironmentVariable("PATH", "$CurrentPath;$InstallPath", "User")
        Write-Success "Added Jorm-RS to user PATH"
    } catch {
        Write-Warning "Failed to add Jorm-RS to PATH: $_"
        Write-Warning "Please add $InstallPath to your PATH manually"
    }
} else {
    Write-Status "Jorm-RS already in PATH"
}

# Create configuration directory
$ConfigDir = Join-Path $env:USERPROFILE ".config\jorm-rs"
if (!(Test-Path $ConfigDir)) {
    Write-Status "Creating configuration directory: $ConfigDir"
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
}

# Create sample configuration
$ConfigFile = Join-Path $ConfigDir "config.toml"
if (!(Test-Path $ConfigFile)) {
    Write-Status "Creating sample configuration..."
    $ConfigContent = @"
# Jorm-RS Configuration
[executor]
max_concurrent_tasks = 4
default_timeout = 300

[scheduler]
max_concurrent_jobs = 10

[ai]
model_provider = "local"
"@
    Set-Content -Path $ConfigFile -Value $ConfigContent
}

# Create desktop shortcut (optional)
$DesktopPath = [Environment]::GetFolderPath("Desktop")
$ShortcutPath = Join-Path $DesktopPath "Jorm-RS.lnk"
if (!(Test-Path $ShortcutPath)) {
    try {
        $WshShell = New-Object -ComObject WScript.Shell
        $Shortcut = $WshShell.CreateShortcut($ShortcutPath)
        $Shortcut.TargetPath = $BinaryPath
        $Shortcut.WorkingDirectory = $InstallPath
        $Shortcut.Description = "Jorm-RS - Pure Rust DAG Engine"
        $Shortcut.Save()
        Write-Success "Desktop shortcut created"
    } catch {
        Write-Warning "Failed to create desktop shortcut: $_"
    }
}

# Create Start Menu shortcut (optional)
$StartMenuPath = [Environment]::GetFolderPath("StartMenu")
$StartMenuShortcut = Join-Path $StartMenuPath "Programs\Jorm-RS.lnk"
$StartMenuDir = Split-Path $StartMenuShortcut -Parent
if (!(Test-Path $StartMenuDir)) {
    New-Item -ItemType Directory -Path $StartMenuDir -Force | Out-Null
}

if (!(Test-Path $StartMenuShortcut)) {
    try {
        $WshShell = New-Object -ComObject WScript.Shell
        $Shortcut = $WshShell.CreateShortcut($StartMenuShortcut)
        $Shortcut.TargetPath = $BinaryPath
        $Shortcut.WorkingDirectory = $InstallPath
        $Shortcut.Description = "Jorm-RS - Pure Rust DAG Engine"
        $Shortcut.Save()
        Write-Success "Start Menu shortcut created"
    } catch {
        Write-Warning "Failed to create Start Menu shortcut: $_"
    }
}

Write-Success "Jorm-RS installed successfully!"
Write-Status "Installation path: $InstallPath"
Write-Status "Configuration directory: $ConfigDir"
Write-Warning "Please restart your terminal or run: refreshenv"
Write-Status "Run 'jorm-rs --help' to get started"
Write-Status "Run 'jorm-rs setup' to configure your environment"

# Test final installation
Write-Status "Testing final installation..."
try {
    $FinalVersion = & $BinaryPath --version
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Installation verification passed: $FinalVersion"
    } else {
        Write-Warning "Installation verification failed, but installation may still work"
    }
} catch {
    Write-Warning "Could not verify installation, but it may still work"
}





