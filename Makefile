# Makefile complet pour slashsum avec setup Rust
.PHONY: build test clean run help install release setup-rust setup-windows setup-dev build-windows build-all

# Variables
BINARY_NAME=slashsum
VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo "dev")
BUILD_TIME_VAL = $(shell date -u +%Y-%m-%dT%H:%M:%SZ)
GIT_COMMIT_VAL = $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")

# Variables d'environnement pour Rust
export BUILD_VERSION=$(VERSION)
export BUILD_TIME=$(BUILD_TIME_VAL)
export GIT_COMMIT=$(GIT_COMMIT_VAL)

# =============================================================================
# SETUP ET INSTALLATION
# =============================================================================

check-rust: ## Vérifier si Rust est installé
	@if ! command -v rustc >/dev/null 2>&1; then \
		echo "❌ Rust n'est pas installé"; \
		echo "💡 Lancez 'make setup-rust' pour l'installer"; \
		exit 1; \
	else \
		echo "✅ Rust $(shell rustc --version)"; \
	fi

setup-rust: ## Installer Rust via rustup
	@echo "🦀 Installation de Rust..."
	@if ! command -v rustc >/dev/null 2>&1; then \
		echo "📥 Téléchargement et installation de rustup..."; \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
		echo "🔄 Rechargement de l'environnement..."; \
		. ~/.cargo/env; \
		echo "✅ Rust installé avec succès"; \
	else \
		echo "✅ Rust déjà installé"; \
	fi
	@echo "🔧 Configuration des composants..."
	rustup component add clippy rustfmt
	@echo "📋 Versions installées:"
	@rustc --version
	@cargo --version
	@rustup --version

setup-windows: ## Installer les outils pour cross-compilation Windows
	@echo "🪟 Installation des outils Windows..."
	@make check-rust
	@echo "📥 Installation de la target Windows..."
	rustup target add x86_64-pc-windows-gnu
	@echo "🔧 Installation des outils de cross-compilation..."
	@if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then \
		echo "📦 Installation de mingw-w64..."; \
		sudo apt update; \
		sudo apt install -y gcc-mingw-w64-x86-64; \
	else \
		echo "✅ mingw-w64 déjà installé"; \
	fi
	@echo "✅ Setup Windows terminé"

setup-dev: ## Installer tous les outils de développement
	@echo "🛠️  Installation complète de l'environnement de développement..."
	@make setup-rust
	@make setup-windows
	@echo "🔧 Installation des outils additionnels..."
	@if ! command -v cargo-audit >/dev/null 2>&1; then \
		cargo install cargo-audit; \
	else \
		echo "✅ cargo-audit déjà installé"; \
	fi
	@if ! command -v cargo-tarpaulin >/dev/null 2>&1; then \
		cargo install cargo-tarpaulin; \
	else \
		echo "✅ cargo-tarpaulin déjà installé"; \
	fi
	@echo "📋 Résumé de l'installation:"
	@echo "  🦀 Rust: $(shell rustc --version 2>/dev/null || echo 'Non installé')"
	@echo "  📦 Cargo: $(shell cargo --version 2>/dev/null || echo 'Non installé')"
	@echo "  🪟 Windows target: $(shell rustup target list --installed | grep x86_64-pc-windows-gnu || echo 'Non installé')"
	@echo "  🔍 Clippy: $(shell rustup component list --installed | grep clippy || echo 'Non installé')"
	@echo "  🎨 rustfmt: $(shell rustup component list --installed | grep rustfmt || echo 'Non installé')"
	@echo "  🛡️  cargo-audit: $(shell command -v cargo-audit >/dev/null 2>&1 && echo 'Installé' || echo 'Non installé')"
	@echo "  📊 cargo-tarpaulin: $(shell command -v cargo-tarpaulin >/dev/null 2>&1 && echo 'Installé' || echo 'Non installé')"
	@echo "✅ Environnement de développement prêt!"

update-rust: ## Mettre à jour Rust et ses composants
	@echo "🔄 Mise à jour de Rust..."
	rustup update
	rustup component add clippy rustfmt
	@echo "✅ Rust mis à jour"

# =============================================================================
# BUILD
# =============================================================================

build: check-rust ## Compiler le binaire Linux
	@echo "🔨 Building $(BINARY_NAME) version $(VERSION)..."
	@echo "📅 Build time: $(BUILD_TIME_VAL)"
	@echo "🔗 Git commit: $(GIT_COMMIT_VAL)"
	cargo build --release

build-local: check-rust build ## Build avec vérifications pour usage local

build-windows: ## Build pour Windows (avec vérifications)
	@echo "🪟 Building for Windows..."
	@make check-rust
	@if ! rustup target list --installed | grep -q x86_64-pc-windows-gnu; then \
		echo "❌ Windows target manquante"; \
		echo "💡 Lancez 'make setup-windows' pour l'installer"; \
		exit 1; \
	fi
	@if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then \
		echo "❌ Outils de cross-compilation manquants"; \
		echo "💡 Lancez 'make setup-windows' pour les installer"; \
		exit 1; \
	fi
	@export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc && \
	export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++ && \
	export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar && \
	export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc && \
	cargo build --release --target x86_64-pc-windows-gnu
	@echo "✅ Windows build terminé"

build-all: build build-windows ## Build pour toutes les plateformes
	@echo "🎉 All builds completed successfully!"
	@echo "📁 Binaires créés:"
	@echo "  🐧 Linux:   target/release/$(BINARY_NAME)"
	@echo "  🪟 Windows: target/x86_64-pc-windows-gnu/release/$(BINARY_NAME).exe"
	@ls -la target/release/$(BINARY_NAME) 2>/dev/null || echo "  ❌ Linux binary manquant"
	@ls -la target/x86_64-pc-windows-gnu/release/$(BINARY_NAME).exe 2>/dev/null || echo "  ❌ Windows binary manquant"

debug: check-rust ## Compiler en mode debug
	@echo "🔨 Building $(BINARY_NAME) version $(VERSION) (debug)..."
	cargo build

# =============================================================================
# TESTS
# =============================================================================

test: check-rust ## Lancer les tests basiques
	cargo test

test-all: check-rust ## Lancer tous les tests (y compris ignorés)
	@echo "🧪 Tests complets..."
	cargo test -- --include-ignored

test-verbose: check-rust ## Tests avec sortie détaillée
	@echo "🔍 Tests verbeux..."
	cargo test -- --nocapture

test-performance: check-rust ## Tests de performance uniquement
	@echo "⚡ Tests de performance..."
	cargo test --release -- --ignored --nocapture

test-coverage: check-rust ## Tests avec couverture de code
	@echo "📊 Tests avec couverture..."
	@if ! command -v cargo-tarpaulin >/dev/null 2>&1; then \
		echo "📥 Installation de cargo-tarpaulin..."; \
		cargo install cargo-tarpaulin; \
	fi
	cargo tarpaulin --verbose --all-features --workspace --timeout 120

test-windows: build-windows ## Tester le binaire Windows avec Wine
	@echo "🧪 Testing Windows binary..."
	@if command -v wine >/dev/null 2>&1; then \
		echo "🍷 Test avec Wine:"; \
		wine target/x86_64-pc-windows-gnu/release/$(BINARY_NAME).exe --version; \
	else \
		echo "⚠️  Wine non installé - vérification basique:"; \
		file target/x86_64-pc-windows-gnu/release/$(BINARY_NAME).exe; \
	fi

test-files: build ## Tests avec fichiers réels
	@echo "📁 Tests avec fichiers réels..."
	@mkdir -p /tmp/slashsum_test
	@echo "Hello World!" > /tmp/slashsum_test/small.txt
	@dd if=/dev/zero of=/tmp/slashsum_test/medium.bin bs=1M count=1 2>/dev/null
	@touch /tmp/slashsum_test/empty.txt
	@echo "🧪 Test fichier petit..."
	@./target/release/$(BINARY_NAME) /tmp/slashsum_test/small.txt
	@echo "🧪 Test fichier vide..."
	@./target/release/$(BINARY_NAME) /tmp/slashsum_test/empty.txt
	@echo "🧪 Test fichier moyen..."
	@./target/release/$(BINARY_NAME) /tmp/slashsum_test/medium.bin
	@echo "🧪 Test option --save..."
	@./target/release/$(BINARY_NAME) /tmp/slashsum_test/small.txt --save
	@rm -rf /tmp/slashsum_test
	@echo "✅ Tests fichiers terminés"

# =============================================================================
# QUALITÉ DE CODE
# =============================================================================

lint: check-rust ## Vérifications avec Clippy
	@echo "🔍 Vérifications Clippy..."
	cargo clippy -- -D warnings

fmt: check-rust ## Vérifier le formatage du code
	@echo "🎨 Vérification du formatage..."
	cargo fmt -- --check

fmt-fix: check-rust ## Corriger le formatage du code
	@echo "🎨 Correction du formatage..."
	cargo fmt

check: check-rust ## Vérification rapide de compilation
	@echo "🔧 Vérification de compilation..."
	cargo check

audit: check-rust ## Audit de sécurité
	@echo "🛡️  Audit de sécurité..."
	@if ! command -v cargo-audit >/dev/null 2>&1; then \
		echo "📥 Installation de cargo-audit..."; \
		cargo install cargo-audit; \
	fi
	cargo audit

ci: lint fmt test audit ## Pipeline CI complète
	@echo "✅ Pipeline CI terminée avec succès"

# =============================================================================
# DOCUMENTATION ET UTILITAIRES
# =============================================================================

doc: check-rust ## Générer la documentation
	@echo "📚 Génération de la documentation..."
	cargo doc --no-deps --document-private-items --open

clean: ## Nettoyer les fichiers de build
	cargo clean

run: check-rust ## Compiler et exécuter
	cargo run

install: build ## Installer le binaire dans ~/.cargo/bin
	cp target/release/$(BINARY_NAME) ~/.cargo/bin/

uninstall: ## Désinstaller le binaire de ~/.cargo/bin
	@echo "🗑️  Désinstallation de $(BINARY_NAME)..."
	@if [ -f ~/.cargo/bin/$(BINARY_NAME) ]; then \
		rm ~/.cargo/bin/$(BINARY_NAME); \
		echo "✅ $(BINARY_NAME) désinstallé avec succès"; \
	else \
		echo "⚠️  $(BINARY_NAME) n'est pas installé dans ~/.cargo/bin"; \
	fi

release: check-rust ## Build optimisé pour release
	@echo "🚀 Building release $(BINARY_NAME) version $(VERSION)..."
	cargo build --release --target x86_64-unknown-linux-gnu

# Afficher les informations de version
version: ## Afficher les informations de version
	@echo "📋 Informations de version:"
	@echo "  Version: $(VERSION)"
	@echo "  Build time: $(BUILD_TIME_VAL)"
	@echo "  Git commit: $(GIT_COMMIT_VAL)"

status: ## Afficher le statut de l'environnement
	@echo "📊 Statut de l'environnement de développement:"
	@echo "🦀 Rust:"
	@echo "  Version: $(shell rustc --version 2>/dev/null || echo '❌ Non installé')"
	@echo "  Cargo: $(shell cargo --version 2>/dev/null || echo '❌ Non installé')"
	@echo "🎯 Targets installées:"
	@rustup target list --installed 2>/dev/null | sed 's/^/  /' || echo "  ❌ rustup non disponible"
	@echo "🔧 Composants:"
	@rustup component list --installed 2>/dev/null | sed 's/^/  /' || echo "  ❌ rustup non disponible"
	@echo "🛠️  Outils additionnels:"
	@echo "  cargo-audit: $(shell command -v cargo-audit >/dev/null 2>&1 && echo '✅ Installé' || echo '❌ Non installé')"
	@echo "  cargo-tarpaulin: $(shell command -v cargo-tarpaulin >/dev/null 2>&1 && echo '✅ Installé' || echo '❌ Non installé')"
	@echo "🪟 Cross-compilation Windows:"
	@echo "  mingw-w64: $(shell command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1 && echo '✅ Installé' || echo '❌ Non installé')"

help: ## Afficher cette aide
	@echo "🛠️  Makefile pour $(BINARY_NAME)"
	@echo ""
	@echo "📋 Commandes disponibles:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "🚀 Commandes de démarrage rapide:"
	@echo "  make setup-dev          # Installation complète de l'environnement"
	@echo "  make build-all          # Build Linux + Windows"
	@echo "  make ci                 # Pipeline complète (lint + test + audit)"
	@echo ""
	@echo "📊 Commandes d'information:"
	@echo "  make status             # Statut de l'environnement"
	@echo "  make version            # Informations de version"
