#!/bin/bash
set -euo pipefail

echo "🔧 DupliFind - Mac Development Setup"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if a command exists
check_command() {
    if command -v "$1" &> /dev/null; then
        echo -e "${GREEN}✓${NC} $1 is installed"
        return 0
    else
        echo -e "${RED}✗${NC} $1 is not installed"
        return 1
    fi
}

# Check Xcode Command Line Tools
echo ""
echo "Checking Xcode Command Line Tools..."
if xcode-select -p &> /dev/null; then
    echo -e "${GREEN}✓${NC} Xcode Command Line Tools installed"
else
    echo -e "${YELLOW}Installing Xcode Command Line Tools...${NC}"
    xcode-select --install
    echo "Please complete the installation dialog, then run this script again."
    exit 0
fi

# Check Homebrew
echo ""
echo "Checking Homebrew..."
if ! check_command brew; then
    echo -e "${YELLOW}Homebrew not found.${NC}"
    echo "To install Homebrew, please run the following command manually:"
    echo ""
    echo '  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"'
    echo ""
    echo "Then run this script again."
    exit 1
fi

# Add Homebrew to path for Apple Silicon if not already present
if [[ $(uname -m) == 'arm64' ]]; then
    if ! grep -qF '/opt/homebrew/bin/brew shellenv' ~/.zprofile 2>/dev/null; then
        echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
    fi
    eval "$(/opt/homebrew/bin/brew shellenv)" 2>/dev/null || true
fi

# Check Node.js
echo ""
echo "Checking Node.js..."
if check_command node; then
    NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
    if [[ "$NODE_VERSION" =~ ^[0-9]+$ ]] && [ "$NODE_VERSION" -ge 20 ]; then
        echo -e "${GREEN}✓${NC} Node.js version is 20+"
    else
        echo -e "${YELLOW}Node.js version is below 20. Installing latest LTS...${NC}"
        brew install node@20
    fi
else
    echo -e "${YELLOW}Installing Node.js...${NC}"
    brew install node@20
fi

# Check Rust
echo ""
echo "Checking Rust..."
if check_command rustc; then
    RUST_VERSION=$(rustc --version)
    echo -e "${GREEN}✓${NC} Rust: $RUST_VERSION"
else
    echo -e "${YELLOW}Rust not found.${NC}"
    echo "To install Rust, please run the following command manually:"
    echo ""
    echo '  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh'
    echo ""
    echo "After installation completes, run this script again."
    exit 1
fi

# Update Rust to latest stable
echo ""
echo "Updating Rust to latest stable..."
rustup update stable
rustup default stable

# Check for required Rust targets (for cross-compilation if needed)
echo ""
echo "Adding required Rust targets..."
if ! rustup target add aarch64-apple-darwin x86_64-apple-darwin 2>/dev/null; then
    echo -e "${YELLOW}Warning: Could not add some Rust targets. Cross-compilation may not work.${NC}"
fi

# Install Tauri CLI
echo ""
echo "Checking Tauri CLI..."
if cargo install --list | grep -q "tauri-cli"; then
    echo -e "${GREEN}✓${NC} Tauri CLI is installed"
else
    echo -e "${YELLOW}Installing Tauri CLI...${NC}"
    cargo install tauri-cli@^2
fi

# Verify all installations
echo ""
echo "======================================"
echo "Verification"
echo "======================================"
echo ""

ALL_GOOD=true

check_command node || ALL_GOOD=false
check_command npm || ALL_GOOD=false
check_command rustc || ALL_GOOD=false
check_command cargo || ALL_GOOD=false

if [ "$ALL_GOOD" = true ]; then
    echo ""
    echo -e "${GREEN}✓ All prerequisites are installed!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Run 'npm install' to install Node dependencies"
    echo "  2. Run 'npm run tauri dev' to start development"
    echo ""
else
    echo ""
    echo -e "${RED}Some prerequisites are missing. Please install them and run this script again.${NC}"
    exit 1
fi
