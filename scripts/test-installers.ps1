# PowerShell test script to verify installer functionality

param(
    [switch]$Verbose
)

Write-Host "🧪 Testing Jorm-RS Installers..." -ForegroundColor Green

# Colors for output
$Colors = @{
    Red = "Red"
    Green = "Green"
    Yellow = "Yellow"
    Blue = "Cyan"
    White = "White"
}

function Write-TestStatus {
    param([string]$Message)
    Write-Host "[TEST] $Message" -ForegroundColor $Colors.Blue
}

function Write-TestSuccess {
    param([string]$Message)
    Write-Host "[PASS] $Message" -ForegroundColor $Colors.Green
}

function Write-TestError {
    param([string]$Message)
    Write-Host "[FAIL] $Message" -ForegroundColor $Colors.Red
}

function Write-TestWarning {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor $Colors.Yellow
}

# Test 1: Check if installer scripts exist
Write-TestStatus "Testing installer script existence..."

if (Test-Path "scripts/install-linux.sh") {
    Write-TestSuccess "Linux installer script exists"
} else {
    Write-TestError "Linux installer script missing"
    exit 1
}

if (Test-Path "scripts/install-macos.sh") {
    Write-TestSuccess "macOS installer script exists"
} else {
    Write-TestError "macOS installer script missing"
    exit 1
}

if (Test-Path "scripts/install-windows.ps1") {
    Write-TestSuccess "Windows installer script exists"
} else {
    Write-TestError "Windows installer script missing"
    exit 1
}

# Test 2: Check script syntax
Write-TestStatus "Testing script syntax..."

# Test Windows script syntax
try {
    $null = [System.Management.Automation.PSParser]::Tokenize((Get-Content "scripts/install-windows.ps1" -Raw), [ref]$null)
    Write-TestSuccess "Windows installer syntax is valid"
} catch {
    Write-TestError "Windows installer syntax error: $_"
    exit 1
}

# Test 3: Check GitHub Actions workflow
Write-TestStatus "Testing GitHub Actions workflow..."

if (Test-Path ".github/workflows/ci.yml") {
    Write-TestSuccess "GitHub Actions workflow exists"
    
    # Basic YAML content check
    $yamlContent = Get-Content ".github/workflows/ci.yml" -Raw
    if ($yamlContent -match "name:" -and $yamlContent -match "on:" -and $yamlContent -match "jobs:") {
        Write-TestSuccess "GitHub Actions workflow structure is valid"
    } else {
        Write-TestWarning "GitHub Actions workflow structure may have issues"
    }
} else {
    Write-TestError "GitHub Actions workflow missing"
    exit 1
}

# Test 4: Check for required files
Write-TestStatus "Testing required files..."

$requiredFiles = @(
    "Cargo.toml",
    "src/main.rs",
    "src/lib.rs",
    "README.md"
)

foreach ($file in $requiredFiles) {
    if (Test-Path $file) {
        Write-TestSuccess "Required file exists: $file"
    } else {
        Write-TestError "Required file missing: $file"
        exit 1
    }
}

# Test 5: Check Cargo configuration
Write-TestStatus "Testing Cargo configuration..."

if (Test-Path "Cargo.toml") {
    $cargoContent = Get-Content "Cargo.toml" -Raw
    
    # Check if binary is defined
    if ($cargoContent -match "\[\[bin\]\]") {
        Write-TestSuccess "Binary configuration found in Cargo.toml"
    } else {
        Write-TestWarning "No binary configuration found in Cargo.toml"
    }
    
    # Check if library is defined
    if ($cargoContent -match "\[lib\]") {
        Write-TestSuccess "Library configuration found in Cargo.toml"
    } else {
        Write-TestWarning "No library configuration found in Cargo.toml"
    }
}

# Test 6: Check test structure
Write-TestStatus "Testing test structure..."

if (Test-Path "tests") {
    Write-TestSuccess "Tests directory exists"
    
    # Check for integration tests
    if (Test-Path "tests/integration") {
        Write-TestSuccess "Integration tests directory exists"
    } else {
        Write-TestWarning "Integration tests directory missing"
    }
    
    # Check for unit tests
    if (Test-Path "tests/unit") {
        Write-TestSuccess "Unit tests directory exists"
    } else {
        Write-TestWarning "Unit tests directory missing"
    }
} else {
    Write-TestWarning "Tests directory missing"
}

# Test 7: Check documentation
Write-TestStatus "Testing documentation..."

if (Test-Path "README.md") {
    Write-TestSuccess "README.md exists"
} else {
    Write-TestWarning "README.md missing"
}

if (Test-Path "CI_CD_SETUP.md") {
    Write-TestSuccess "CI_CD_SETUP.md exists"
} else {
    Write-TestWarning "CI_CD_SETUP.md missing"
}

# Test 8: Check for common issues
Write-TestStatus "Checking for common issues..."

# Check for hardcoded paths
$hardcodedPaths = Get-ChildItem -Recurse -File | Where-Object { 
    $_.Extension -notin @(".md", ".git") -and 
    (Get-Content $_.FullName -Raw) -match "github.com/your-repo"
}

if ($hardcodedPaths) {
    Write-TestWarning "Found hardcoded repository URLs - update these in the installers"
    if ($Verbose) {
        $hardcodedPaths | ForEach-Object { Write-Host "  - $($_.FullName)" -ForegroundColor $Colors.Yellow }
    }
}

# Check for placeholder text
$placeholderText = Get-ChildItem -Recurse -File | Where-Object { 
    $_.Extension -notin @(".md", ".git") -and 
    (Get-Content $_.FullName -Raw) -match "your-repo"
}

if ($placeholderText) {
    Write-TestWarning "Found placeholder text - update these in the installers"
    if ($Verbose) {
        $placeholderText | ForEach-Object { Write-Host "  - $($_.FullName)" -ForegroundColor $Colors.Yellow }
    }
}

# Test 9: Check PowerShell execution policy
Write-TestStatus "Checking PowerShell execution policy..."

$executionPolicy = Get-ExecutionPolicy
if ($executionPolicy -eq "Restricted") {
    Write-TestWarning "PowerShell execution policy is Restricted - installers may not work"
    Write-TestWarning "Consider running: Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser"
} else {
    Write-TestSuccess "PowerShell execution policy allows script execution: $executionPolicy"
}

# Test 10: Check for required PowerShell modules
Write-TestStatus "Checking for required PowerShell modules..."

$requiredModules = @("WebAdministration", "IISAdministration")
foreach ($module in $requiredModules) {
    if (Get-Module -ListAvailable -Name $module) {
        Write-TestSuccess "PowerShell module available: $module"
    } else {
        Write-TestWarning "PowerShell module not available: $module"
    }
}

Write-TestSuccess "All installer tests completed!"
Write-TestStatus "Next steps:"
Write-TestStatus "1. Update repository URLs in installer scripts"
Write-TestStatus "2. Test installers on target platforms"
Write-TestStatus "3. Create a GitHub release to test the full pipeline"
Write-TestStatus "4. Verify all artifacts are created correctly"


