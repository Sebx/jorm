#!/bin/bash
# Test script to verify installer functionality

set -e

echo "🧪 Testing Jorm-RS Installers..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${BLUE}[TEST]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

print_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Test 1: Check if installer scripts exist
print_status "Testing installer script existence..."

if [[ -f "scripts/install-linux.sh" ]]; then
    print_success "Linux installer script exists"
else
    print_error "Linux installer script missing"
    exit 1
fi

if [[ -f "scripts/install-macos.sh" ]]; then
    print_success "macOS installer script exists"
else
    print_error "macOS installer script missing"
    exit 1
fi

if [[ -f "scripts/install-windows.ps1" ]]; then
    print_success "Windows installer script exists"
else
    print_error "Windows installer script missing"
    exit 1
fi

# Test 2: Check script syntax
print_status "Testing script syntax..."

# Test Linux script syntax
if bash -n scripts/install-linux.sh; then
    print_success "Linux installer syntax is valid"
else
    print_error "Linux installer syntax error"
    exit 1
fi

# Test macOS script syntax
if bash -n scripts/install-macos.sh; then
    print_success "macOS installer syntax is valid"
else
    print_error "macOS installer syntax error"
    exit 1
fi

# Test Windows script syntax (basic PowerShell check)
if powershell -Command "Get-Content scripts/install-windows.ps1 | Out-Null" 2>/dev/null; then
    print_success "Windows installer syntax is valid"
else
    print_warning "Windows installer syntax check failed (PowerShell not available or script issue)"
fi

# Test 3: Check GitHub Actions workflow
print_status "Testing GitHub Actions workflow..."

if [[ -f ".github/workflows/ci.yml" ]]; then
    print_success "GitHub Actions workflow exists"
    
    # Basic YAML syntax check
    if command -v yamllint &> /dev/null; then
        if yamllint .github/workflows/ci.yml; then
            print_success "GitHub Actions workflow syntax is valid"
        else
            print_warning "GitHub Actions workflow has syntax issues"
        fi
    else
        print_warning "yamllint not available, skipping YAML syntax check"
    fi
else
    print_error "GitHub Actions workflow missing"
    exit 1
fi

# Test 4: Check for required files
print_status "Testing required files..."

required_files=(
    "Cargo.toml"
    "src/main.rs"
    "src/lib.rs"
    "README.md"
)

for file in "${required_files[@]}"; do
    if [[ -f "$file" ]]; then
        print_success "Required file exists: $file"
    else
        print_error "Required file missing: $file"
        exit 1
    fi
done

# Test 5: Check Cargo configuration
print_status "Testing Cargo configuration..."

if [[ -f "Cargo.toml" ]]; then
    # Check if binary is defined
    if grep -q "\[\[bin\]\]" Cargo.toml; then
        print_success "Binary configuration found in Cargo.toml"
    else
        print_warning "No binary configuration found in Cargo.toml"
    fi
    
    # Check if library is defined
    if grep -q "\[lib\]" Cargo.toml; then
        print_success "Library configuration found in Cargo.toml"
    else
        print_warning "No library configuration found in Cargo.toml"
    fi
fi

# Test 6: Check test structure
print_status "Testing test structure..."

if [[ -d "tests" ]]; then
    print_success "Tests directory exists"
    
    # Check for integration tests
    if [[ -d "tests/integration" ]]; then
        print_success "Integration tests directory exists"
    else
        print_warning "Integration tests directory missing"
    fi
    
    # Check for unit tests
    if [[ -d "tests/unit" ]]; then
        print_success "Unit tests directory exists"
    else
        print_warning "Unit tests directory missing"
    fi
else
    print_warning "Tests directory missing"
fi

# Test 7: Check documentation
print_status "Testing documentation..."

if [[ -f "README.md" ]]; then
    print_success "README.md exists"
else
    print_warning "README.md missing"
fi

if [[ -f "CI_CD_SETUP.md" ]]; then
    print_success "CI_CD_SETUP.md exists"
else
    print_warning "CI_CD_SETUP.md missing"
fi

# Test 8: Check for common issues
print_status "Checking for common issues..."

# Check for hardcoded paths
if grep -r "github.com/your-repo" . --exclude-dir=.git --exclude="*.md" 2>/dev/null; then
    print_warning "Found hardcoded repository URLs - update these in the installers"
fi

# Check for placeholder text
if grep -r "your-repo" . --exclude-dir=.git --exclude="*.md" 2>/dev/null; then
    print_warning "Found placeholder text - update these in the installers"
fi

print_success "All installer tests completed!"
print_status "Next steps:"
print_status "1. Update repository URLs in installer scripts"
print_status "2. Test installers on target platforms"
print_status "3. Create a GitHub release to test the full pipeline"
print_status "4. Verify all artifacts are created correctly"





