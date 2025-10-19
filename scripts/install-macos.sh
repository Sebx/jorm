#!/bin/bash
set -e

echo "🚀 Installing Jorm-RS on macOS..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [[ $EUID -eq 0 ]]; then
   print_error "This script should not be run as root for security reasons"
   exit 1
fi

# Detect architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64) ARCH="x86_64" ;;
    arm64) ARCH="aarch64" ;;
    *) print_error "Unsupported architecture: $ARCH"; exit 1 ;;
esac

print_status "Detected architecture: $ARCH"

# Check for required dependencies
print_status "Checking dependencies..."

# Check for curl
if ! command -v curl &> /dev/null; then
    print_error "curl is required but not installed. Please install curl first."
    exit 1
fi

# Check for tar
if ! command -v tar &> /dev/null; then
    print_error "tar is required but not installed. Please install tar first."
    exit 1
fi

# Check for Homebrew (optional but recommended)
if ! command -v brew &> /dev/null; then
    print_warning "Homebrew not found. Consider installing it for better package management."
    print_warning "Visit: https://brew.sh"
fi

# Check for Python (optional but recommended)
if ! command -v python3 &> /dev/null; then
    print_warning "Python3 not found. Some features may not work properly."
    print_warning "Consider installing Python3: brew install python3"
fi

# Set installation directory
INSTALL_DIR="/usr/local/bin"
TEMP_DIR=$(mktemp -d)

# Cleanup function
cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

# Download URL
DOWNLOAD_URL="https://github.com/${{ github.repository }}/releases/latest/download/jorm-rs-macos-${ARCH}.tar.gz"

print_status "Downloading Jorm-RS from: $DOWNLOAD_URL"

# Download and extract
cd "$TEMP_DIR"
if ! curl -L "$DOWNLOAD_URL" -o jorm-rs.tar.gz; then
    print_error "Failed to download Jorm-RS"
    exit 1
fi

if ! tar -xzf jorm-rs.tar.gz; then
    print_error "Failed to extract Jorm-RS"
    exit 1
fi

# Verify binary
if [[ ! -f "jorm-rs" ]]; then
    print_error "Jorm-RS binary not found in archive"
    exit 1
fi

# Make binary executable
chmod +x jorm-rs

# Test binary
print_status "Testing Jorm-RS binary..."
if ! ./jorm-rs --version; then
    print_error "Jorm-RS binary test failed"
    exit 1
fi

# Install to system directory
print_status "Installing Jorm-RS to $INSTALL_DIR..."
if ! sudo cp jorm-rs "$INSTALL_DIR/"; then
    print_error "Failed to install Jorm-RS to $INSTALL_DIR"
    exit 1
fi

# Verify installation
if ! command -v jorm-rs &> /dev/null; then
    print_warning "Jorm-RS installed but not found in PATH"
    print_warning "You may need to restart your terminal or run: source ~/.zshrc"
else
    print_success "Jorm-RS installed successfully!"
    print_success "Version: $(jorm-rs --version)"
fi

# Create configuration directory
CONFIG_DIR="$HOME/.config/jorm-rs"
if [[ ! -d "$CONFIG_DIR" ]]; then
    print_status "Creating configuration directory: $CONFIG_DIR"
    mkdir -p "$CONFIG_DIR"
fi

# Create sample configuration
if [[ ! -f "$CONFIG_DIR/config.toml" ]]; then
    print_status "Creating sample configuration..."
    cat > "$CONFIG_DIR/config.toml" << 'EOF'
# Jorm-RS Configuration
[executor]
max_concurrent_tasks = 4
default_timeout = 300

[scheduler]
max_concurrent_jobs = 10

[ai]
model_provider = "local"
EOF
fi

# Check for Homebrew and suggest installation of dependencies
if command -v brew &> /dev/null; then
    print_status "Checking for recommended dependencies via Homebrew..."
    
    # Check for Python
    if ! command -v python3 &> /dev/null; then
        print_warning "Python3 not found. Installing via Homebrew..."
        if brew install python3; then
            print_success "Python3 installed successfully"
        else
            print_warning "Failed to install Python3 via Homebrew"
        fi
    fi
fi

print_success "Installation completed!"
print_status "Configuration directory: $CONFIG_DIR"
print_status "Run 'jorm-rs --help' to get started"
print_status "Run 'jorm-rs setup' to configure your environment"





