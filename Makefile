# Complete Makefile for slashsum with Rust setup
.PHONY: build test clean run help install release setup-rust setup-windows setup-dev build-windows build-all

# Variables
BINARY_NAME=slashsum
VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo "dev")
BUILD_TIME_VAL = $(shell date -u +%Y-%m-%dT%H:%M:%SZ)
GIT_COMMIT_VAL = $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")

# Environment variables for Rust
export BUILD_VERSION=$(VERSION)
export BUILD_TIME=$(BUILD_TIME_VAL)
export GIT_COMMIT=$(GIT_COMMIT_VAL)

# =============================================================================
# SETUP AND INSTALLATION
# =============================================================================

check-rust: ## Check if Rust is installed
	@if ! command -v rustc >/dev/null 2>&1; then \
		echo "❌ Rust is not installed"; \
		echo "💡 Run 'make setup-rust' to install it"; \
		exit 1; \
	else \
		echo "✅ Rust $(shell rustc --version)"; \
	fi

setup-rust: ## Install Rust via rustup
	@echo "🦀 Installing Rust..."
	@if ! command -v rustc >/dev/null 2>&1; then \
		echo "📥 Downloading and installing rustup..."; \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
		echo "🔄 Reloading environment..."; \
		. ~/.cargo/env; \
		echo "✅ Rust installed successfully"; \
	else \
		echo "✅ Rust already installed"; \
	fi
	@echo "🔧 Configuring components..."
	rustup component add clippy rustfmt
	@echo "📋 Installed versions:"
	@rustc --version
	@cargo --version
	@rustup --version

setup-windows: ## Install tools for Windows cross-compilation
	@echo "🪟 Installing Windows tools..."
	@make check-rust
	@echo "📥 Installing Windows target..."
	rustup target add x86_64-pc-windows-gnu
	@echo "🔧 Installing cross-compilation tools..."
	@if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then \
		echo "📦 Installing mingw-w64..."; \
		sudo apt update; \
		sudo apt install -y gcc-mingw-w64-x86-64; \
	else \
		echo "✅ mingw-w64 already installed"; \
	fi
	@echo "✅ Windows setup complete"

setup-dev: ## Install all development tools
	@echo "🛠️  Full development environment installation..."
	@make setup-rust
	@make setup-windows
	@echo "🔧 Installing additional tools..."
	@if ! command -v cargo-audit >/dev/null 2>&1; then \
		cargo install cargo-audit; \
	else \
		echo "✅ cargo-audit already installed"; \
	fi
	@if ! command -v cargo-tarpaulin >/dev/null 2>&1; then \
		cargo install cargo-tarpaulin; \
	else \
		echo "✅ cargo-tarpaulin already installed"; \
	fi
	@echo "📋 Installation summary:"
	@echo "  🦀 Rust: $(shell rustc --version 2>/dev/null || echo 'Not installed')"
	@echo "  📦 Cargo: $(shell cargo --version 2>/dev/null || echo 'Not installed')"
	@echo "  🪟 Windows target: $(shell rustup target list --installed | grep x86_64-pc-windows-gnu || echo 'Not installed')"
	@echo "  🔍 Clippy: $(shell rustup component list --installed | grep clippy || echo 'Not installed')"
	@echo "  🎨 rustfmt: $(shell rustup component list --installed | grep rustfmt || echo 'Not installed')"
	@echo "  🛡️  cargo-audit: $(shell command -v cargo-audit >/dev/null 2>&1 && echo 'Installed' || echo 'Not installed')"
	@echo "  📊 cargo-tarpaulin: $(shell command -v cargo-tarpaulin >/dev/null 2>&1 && echo 'Installed' || echo 'Not installed')"
	@echo "✅ Development environment ready!"

update-rust: ## Update Rust and its components
	@echo "🔄 Updating Rust..."
	rustup update
	rustup component add clippy rustfmt
	@echo "✅ Rust updated"

# =============================================================================
# BUILD
# =============================================================================

build: check-rust ## Build Linux binary
	@echo "🔨 Building $(BINARY_NAME) version $(VERSION)..."
	@echo "📅 Build time: $(BUILD_TIME_VAL)"
	@echo "🔗 Git commit: $(GIT_COMMIT_VAL)"
	cargo build --release

build-local: check-rust build ## Build with checks for local use

build-windows: ## Build for Windows (with checks)
	@echo "🪟 Building for Windows..."
	@make check-rust
	@if ! rustup target list --installed | grep -q x86_64-pc-windows-gnu; then \
		echo "❌ Windows target missing"; \
		echo "💡 Run 'make setup-windows' to install it"; \
		exit 1; \
	fi
	@if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then \
		echo "❌ Cross-compilation tools missing"; \
		echo "💡 Run 'make setup-windows' to install them"; \
		exit 1; \
	fi
	@export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc && \
	export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++ && \
	export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar && \
	export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc && \
	cargo build --release --target x86_64-pc-windows-gnu
	@echo "✅ Windows build complete"

build-all: build build-windows ## Build for all platforms
	@echo "🎉 All builds completed successfully!"
	@echo "📁 Binaries created:"
	@echo "  🐧 Linux:   target/release/$(BINARY_NAME)"
	@echo "  🪟 Windows: target/x86_64-pc-windows-gnu/release/$(BINARY_NAME).exe"
	@ls -la target/release/$(BINARY_NAME) 2>/dev/null || echo "  ❌ Linux binary missing"
	@ls -la target/x86_64-pc-windows-gnu/release/$(BINARY_NAME).exe 2>/dev/null || echo "  ❌ Windows binary missing"

debug: check-rust ## Build in debug mode
	@echo "🔨 Building $(BINARY_NAME) version $(VERSION) (debug)..."
	cargo build

# =============================================================================
# TESTS
# =============================================================================

test: check-rust ## Run basic tests
	cargo test

test-all: check-rust ## Run all tests (including ignored)
	@echo "🧪 Full tests..."
	cargo test -- --include-ignored

test-verbose: check-rust ## Tests with detailed output
	@echo "🔍 Verbose tests..."
	cargo test -- --nocapture

test-performance: check-rust ## Performance tests only
	@echo "⚡ Performance tests..."
	cargo test --release -- --ignored --nocapture

test-coverage: check-rust ## Tests with code coverage
	@echo "📊 Tests with coverage..."
	@if ! command -v cargo-tarpaulin >/dev/null 2>&1; then \
		echo "📥 Installing cargo-tarpaulin..."; \
		cargo install cargo-tarpaulin; \
	fi
	cargo tarpaulin --verbose --all-features --workspace --timeout 120

test-windows: build-windows ## Test Windows binary with Wine
	@echo "🧪 Testing Windows binary..."
	@if command -v wine >/dev/null 2>&1; then \
		echo "🍷 Testing with Wine:"; \
		wine target/x86_64-pc-windows-gnu/release/$(BINARY_NAME).exe --version; \
	else \
		echo "⚠️  Wine not installed - basic check:"; \
		file target/x86_64-pc-windows-gnu/release/$(BINARY_NAME).exe; \
	fi

test-files: build ## Tests with real files
	@echo "📁 Tests with real files..."
	@mkdir -p /tmp/slashsum_test
	@echo "Hello World!" > /tmp/slashsum_test/small.txt
	@dd if=/dev/zero of=/tmp/slashsum_test/medium.bin bs=1M count=1 2>/dev/null
	@touch /tmp/slashsum_test/empty.txt
	@echo "🧪 Testing small file..."
	@./target/release/$(BINARY_NAME) /tmp/slashsum_test/small.txt
	@echo "🧪 Testing empty file..."
	@./target/release/$(BINARY_NAME) /tmp/slashsum_test/empty.txt
	@echo "🧪 Testing medium file..."
	@./target/release/$(BINARY_NAME) /tmp/slashsum_test/medium.bin
	@echo "🧪 Testing --save option..."
	@./target/release/$(BINARY_NAME) /tmp/slashsum_test/small.txt --save
	@rm -rf /tmp/slashsum_test
	@echo "✅ File tests complete"

# =============================================================================
# CODE QUALITY
# =============================================================================

lint: check-rust ## Run Clippy checks
	@echo "🔍 Running Clippy checks..."
	cargo clippy -- -D warnings

fmt: check-rust ## Check code formatting
	@echo "🎨 Checking code formatting..."
	cargo fmt -- --check

fmt-fix: check-rust ## Fix code formatting
	@echo "🎨 Fixing code formatting..."
	cargo fmt

check: check-rust ## Quick compilation check
	@echo "🔧 Checking compilation..."
	cargo check

audit: check-rust ## Security audit
	@echo "🛡️  Security audit..."
	@if ! command -v cargo-audit >/dev/null 2>&1; then \
		echo "📥 Installing cargo-audit..."; \
		cargo install cargo-audit; \
	fi
	cargo audit

ci: lint fmt test audit ## Full CI pipeline
	@echo "✅ CI pipeline completed successfully"

# =============================================================================
# DOCUMENTATION AND UTILITIES
# =============================================================================

doc: check-rust ## Generate documentation
	@echo "📚 Generating documentation..."
	cargo doc --no-deps --document-private-items --open

clean: ## Clean build files
	cargo clean

run: check-rust ## Build and run
	cargo run

install: build ## Install binary to ~/.cargo/bin
	cp target/release/$(BINARY_NAME) ~/.cargo/bin/

uninstall: ## Uninstall binary from ~/.cargo/bin
	@echo "🗑️  Uninstalling $(BINARY_NAME)..."
	@if [ -f ~/.cargo/bin/$(BINARY_NAME) ]; then \
		rm ~/.cargo/bin/$(BINARY_NAME); \
		echo "✅ $(BINARY_NAME) uninstalled successfully"; \
	else \
		echo "⚠️  $(BINARY_NAME) is not installed in ~/.cargo/bin"; \
	fi

release: check-rust ## Optimized release build
	@echo "🚀 Building release $(BINARY_NAME) version $(VERSION)..."
	cargo build --release --target x86_64-unknown-linux-gnu

# Display version information
version: ## Display version information
	@echo "📋 Version information:"
	@echo "  Version: $(VERSION)"
	@echo "  Build time: $(BUILD_TIME_VAL)"
	@echo "  Git commit: $(GIT_COMMIT_VAL)"

status: ## Display environment status
	@echo "📊 Development environment status:"
	@echo "🦀 Rust:"
	@echo "  Version: $(shell rustc --version 2>/dev/null || echo '❌ Not installed')"
	@echo "  Cargo: $(shell cargo --version 2>/dev/null || echo '❌ Not installed')"
	@echo "🎯 Installed targets:"
	@rustup target list --installed 2>/dev/null | sed 's/^/  /' || echo "  ❌ rustup not available"
	@echo "🔧 Components:"
	@rustup component list --installed 2>/dev/null | sed 's/^/  /' || echo "  ❌ rustup not available"
	@echo "🛠️  Additional tools:"
	@echo "  cargo-audit: $(shell command -v cargo-audit >/dev/null 2>&1 && echo '✅ Installed' || echo '❌ Not installed')"
	@echo "  cargo-tarpaulin: $(shell command -v cargo-tarpaulin >/dev/null 2>&1 && echo '✅ Installed' || echo '❌ Not installed')"
	@echo "🪟 Windows cross-compilation:"
	@echo "  mingw-w64: $(shell command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1 && echo '✅ Installed' || echo '❌ Not installed')"

help: ## Display this help
	@echo "🛠️  Makefile for $(BINARY_NAME)"
	@echo ""
	@echo "📋 Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "🚀 Quick start commands:"
	@echo "  make setup-dev          # Full environment installation"
	@echo "  make build-all          # Build Linux + Windows"
	@echo "  make ci                 # Full pipeline (lint + test + audit)"
	@echo ""
	@echo "📊 Information commands:"
	@echo "  make status             # Environment status"
	@echo "  make version            # Version information"
