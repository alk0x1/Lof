.PHONY: help build test clean install check lint format
.PHONY: test-unit test-parser test-typecheck test-compile test-integration test-all
.PHONY: verify verify-quick verify-all
.DEFAULT_GOAL := help

BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[1;33m
NC := \033[0m
CARGO_HOME ?= $(HOME)/.cargo

help:
	@echo -e "$(BLUE)Lof Language - Development Commands$(NC)"
	@echo "======================================"
	@echo ""
	@echo -e "$(GREEN)Building:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "build|install|clean" | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo -e "$(GREEN)Testing:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "test|check|lint|format" | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo -e "$(GREEN)Verification:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "verify" | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""

build:
	@echo -e "$(BLUE)Building Lof workspace...$(NC)"
	cargo build --all

build-release:
	@echo -e "$(BLUE)Building Lof workspace (release)...$(NC)"
	cargo build --release --all

install: build-release
	@echo -e "$(BLUE)Installing Lof CLI...$(NC)"
	cargo install --force --path lof
	@echo -e "$(GREEN)Lof installed successfully!$(NC)"
	@lof --version
	@echo -e "$(BLUE)Installing Lofit toolkit...$(NC)"
	cargo install --force --path lofit
	@echo -e "$(GREEN)Lofit installed successfully!$(NC)"
	@lofit --version
	@echo -e "$(BLUE)Installing lof-witness-gen utility...$(NC)"
	cargo install --force --path lof-witness-gen
	@echo -e "$(GREEN)lof-witness-gen installed successfully!$(NC)"

clean:
	@echo -e "$(BLUE)Cleaning build artifacts...$(NC)"
	cargo clean
	rm -rf target/
	rm -rf verification/outputs/
	find examples/ -type f \( -name "*.r1cs" -o -name "*.bin" -o -name "*.json" \) -delete
	@echo -e "$(GREEN)Cleaned successfully!$(NC)"

check:
	@echo -e "$(BLUE)Running cargo check...$(NC)"
	cargo check --all

lint:
	@echo -e "$(BLUE)Running clippy...$(NC)"
	cargo clippy --all-targets --all-features -- -D warnings

format:
	@echo -e "$(BLUE)Formatting code...$(NC)"
	cargo fmt --all

format-check:
	@echo -e "$(BLUE)Checking code formatting...$(NC)"
	cargo fmt --all -- --check

test-unit:
	@echo -e "$(BLUE)Running unit tests...$(NC)"
	cargo test --all --verbose

test-fast: check lint format-check test-unit
	@echo ""
	@echo -e "$(GREEN)All fast checks passed!$(NC)"

test-parser: build
	@echo -e "$(BLUE)Running parser tests...$(NC)"
	cd tests/scripts && ./runparsertests.sh

test-typecheck: build
	@echo -e "$(BLUE)Running typechecker tests...$(NC)"
	cd tests/scripts && ./runtypecheckertests.sh

test-compile: build
	@echo -e "$(BLUE)Running compilation tests...$(NC)"
	cd tests/scripts && ./runcompiletests.sh

test-integration: test-parser test-typecheck test-compile
	@echo ""
	@echo -e "$(GREEN)All integration tests passed!$(NC)"

verify-quick: build-release install
	@echo -e "$(BLUE)Running quick verification (multiply circuit)...$(NC)"
	python3 verification/verify.py multiply

verify-all: build-release install
	@echo -e "$(BLUE)Running full verification suite...$(NC)"
	python3 verification/verify.py --all

verify: verify-quick

verify-clean:
	@python3 verification/verify.py --clean

test-all: test-fast test-integration
	@echo ""
	@echo -e "$(GREEN)All tests passed!$(NC)"

ci: test-all
	@echo ""
	@echo -e "$(GREEN)All CI checks passed!$(NC)"

dev: format build test-unit
	@echo -e "$(GREEN)Development cycle complete!$(NC)"

pre-commit: format-check lint test-unit
	@echo -e "$(GREEN)Pre-commit checks passed!$(NC)"

pre-push: test-all
	@echo -e "$(GREEN)Ready to push!$(NC)"

run-example:
	@if [ -z "$(EXAMPLE)" ]; then \
		echo -e "$(YELLOW)Usage: make run-example EXAMPLE=01_functions/functions$(NC)"; \
		exit 1; \
	fi
	@echo -e "$(BLUE)Running example: $(EXAMPLE)$(NC)"
	lof compile examples/$(EXAMPLE).lof

info:
	@echo -e "$(BLUE)Lof Project Information$(NC)"
	@echo "======================="
	@echo ""
	@echo "Workspace members:"
	@cargo metadata --no-deps --format-version 1 | jq -r '.workspace_members[]'
	@echo ""
	@echo "Test structure:"
	@echo "  Unit tests:        $$(find lof/tests -name "*.rs" | wc -l) files"
	@echo "  Integration tests: $$(find tests/integration -name "*.lof" | wc -l) files"
	@echo "  Verification tests: $$(find verification/circuits -name "*.lof" | wc -l) files"
	@echo "  Examples:          $$(find examples -name "*.lof" | wc -l) files"
	@echo ""
	@echo "Current branch: $$(git branch --show-current)"
	@echo "Rust version: $$(rustc --version)"
