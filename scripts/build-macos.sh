#!/bin/bash
set -e

# Heronote macOS Universal Binary Build Script
# Builds for both Apple Silicon (ARM64) and Intel (x86_64)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TAURI_DIR="$PROJECT_ROOT/apps/desktop/src-tauri"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Heronote macOS Universal Binary Build ===${NC}"
echo ""

# Check if rustup targets are installed
check_targets() {
    echo -e "${YELLOW}Checking Rust targets...${NC}"

    if ! rustup target list --installed | grep -q "aarch64-apple-darwin"; then
        echo -e "${YELLOW}Installing aarch64-apple-darwin target...${NC}"
        rustup target add aarch64-apple-darwin
    fi

    if ! rustup target list --installed | grep -q "x86_64-apple-darwin"; then
        echo -e "${YELLOW}Installing x86_64-apple-darwin target...${NC}"
        rustup target add x86_64-apple-darwin
    fi

    echo -e "${GREEN}All targets installed.${NC}"
}

# Build for a specific target
build_target() {
    local target=$1
    echo ""
    echo -e "${YELLOW}Building for ${target}...${NC}"

    cd "$PROJECT_ROOT"
    cargo build --release --target "$target" -p heronote-desktop

    echo -e "${GREEN}Build completed for ${target}.${NC}"
}

# Create Universal Binary using lipo
create_universal() {
    local arm_binary="$PROJECT_ROOT/target/aarch64-apple-darwin/release/heronote-desktop"
    local intel_binary="$PROJECT_ROOT/target/x86_64-apple-darwin/release/heronote-desktop"
    local universal_dir="$PROJECT_ROOT/target/universal-apple-darwin/release"
    local universal_binary="$universal_dir/heronote-desktop"

    echo ""
    echo -e "${YELLOW}Creating Universal Binary...${NC}"

    # Create output directory
    mkdir -p "$universal_dir"

    # Check if both binaries exist
    if [ ! -f "$arm_binary" ]; then
        echo -e "${RED}Error: ARM64 binary not found at $arm_binary${NC}"
        exit 1
    fi

    if [ ! -f "$intel_binary" ]; then
        echo -e "${RED}Error: x86_64 binary not found at $intel_binary${NC}"
        exit 1
    fi

    # Create Universal Binary
    lipo -create -output "$universal_binary" "$arm_binary" "$intel_binary"

    # Verify
    echo ""
    echo -e "${GREEN}Universal Binary created at: $universal_binary${NC}"
    echo "Architectures:"
    lipo -info "$universal_binary"
}

# Build using Tauri for app bundle
build_tauri() {
    local target=$1
    echo ""
    echo -e "${YELLOW}Building Tauri app for ${target}...${NC}"

    cd "$PROJECT_ROOT/apps/desktop"

    if [ -n "$target" ]; then
        pnpm tauri build --target "$target"
    else
        pnpm tauri build
    fi

    echo -e "${GREEN}Tauri build completed.${NC}"
}

# Main script
main() {
    local mode=${1:-"rust"}  # Default to rust-only build

    check_targets

    case $mode in
        "rust")
            # Build Rust binaries only
            build_target "aarch64-apple-darwin"
            build_target "x86_64-apple-darwin"
            create_universal
            ;;
        "tauri-arm")
            # Build Tauri app for ARM64 only
            build_tauri "aarch64-apple-darwin"
            ;;
        "tauri-intel")
            # Build Tauri app for Intel only
            build_tauri "x86_64-apple-darwin"
            ;;
        "tauri-both")
            # Build Tauri apps for both architectures
            build_tauri "aarch64-apple-darwin"
            build_tauri "x86_64-apple-darwin"
            ;;
        "all")
            # Build everything
            build_target "aarch64-apple-darwin"
            build_target "x86_64-apple-darwin"
            create_universal
            build_tauri "aarch64-apple-darwin"
            build_tauri "x86_64-apple-darwin"
            ;;
        *)
            echo "Usage: $0 [rust|tauri-arm|tauri-intel|tauri-both|all]"
            echo ""
            echo "Modes:"
            echo "  rust        - Build Rust binaries and create Universal Binary (default)"
            echo "  tauri-arm   - Build Tauri app for Apple Silicon (ARM64)"
            echo "  tauri-intel - Build Tauri app for Intel (x86_64)"
            echo "  tauri-both  - Build Tauri apps for both architectures"
            echo "  all         - Build everything"
            exit 1
            ;;
    esac

    echo ""
    echo -e "${GREEN}=== Build Complete ===${NC}"
}

main "$@"
