.PHONY: help build test clean install check lint format
.PHONY: test-unit test-parser test-typecheck test-compile test-integration test-all
.PHONY: verify verify-quick verify-all
.DEFAULT_GOAL := help

# Colors for output
BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[1;33m
NC := \033[0m

help: ## Show this help message
	@echo "$(BLUE)Lof Language - Development Commands$(NC)"
	@echo "======================================"
	@echo ""
	@echo "$(GREEN)Building:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "build|install|clean" | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)Testing:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "test|check|lint|format" | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)Verification:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "verify" | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""

# ============================================================================
# Building
# ============================================================================

build: ## Build the project in debug mode
	@echo "$(BLUE)Building Lof workspace...$(NC)"
	cargo build --all

build-release: ## Build the project in release mode
	@echo "$(BLUE)Building Lof workspace (release)...$(NC)"
	cargo build --release --all

install: build-release ## Install the Lof CLI locally
	@echo "$(BLUE)Installing Lof CLI...$(NC)"
	cargo install --path cli
	@echo "$(GREEN)Lof installed successfully!$(NC)"
	@lof --version

clean: ## Clean build artifacts
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	cargo clean
	rm -rf target/
	rm -rf verification/outputs/
	find examples/ -type f \( -name "*.r1cs" -o -name "*.bin" -o -name "*.json" \) -delete
	@echo "$(GREEN)Cleaned successfully!$(NC)"

# ============================================================================
# Code Quality
# ============================================================================

check: ## Run cargo check
	@echo "$(BLUE)Running cargo check...$(NC)"
	cargo check --all

lint: ## Run clippy linter
	@echo "$(BLUE)Running clippy...$(NC)"
	cargo clippy --all-targets --all-features -- -D warnings

format: ## Format code with rustfmt
	@echo "$(BLUE)Formatting code...$(NC)"
	cargo fmt --all

format-check: ## Check if code is formatted
	@echo "$(BLUE)Checking code formatting...$(NC)"
	cargo fmt --all -- --check

# ============================================================================
# Testing - Tier 1 (Fast)
# ============================================================================

test-unit: ## Run Rust unit tests
	@echo "$(BLUE)Running unit tests...$(NC)"
	cargo test --all --verbose

test-fast: check lint format-check test-unit ## Run all fast checks (Tier 1)
	@echo ""
	@echo "$(GREEN)All fast checks passed!$(NC)"

# ============================================================================
# Testing - Tier 2 (Integration)
# ============================================================================

test-parser: build ## Run parser integration tests
	@echo "$(BLUE)Running parser tests...$(NC)"
	cd tests/scripts && ./runparsertests.sh

test-typecheck: build ## Run typechecker integration tests
	@echo "$(BLUE)Running typechecker tests...$(NC)"
	cd tests/scripts && ./runtypecheckertests.sh

test-compile: build ## Run R1CS compilation tests
	@echo "$(BLUE)Running compilation tests...$(NC)"
	cd tests/scripts && ./runcompiletests.sh

test-integration: test-parser test-typecheck test-compile ## Run all integration tests (Tier 2)
	@echo ""
	@echo "$(GREEN)All integration tests passed!$(NC)"

# ============================================================================
# Testing - Tier 3 (Verification)
# ============================================================================

verify-quick: build-release install ## Quick verification smoke test
	@echo "$(BLUE)Running quick verification (multiply circuit)...$(NC)"
	cd verification/circuits/01_basic && \
		bash ../../scripts/compile_both.sh multiply && \
		echo "$(GREEN)Quick verification passed!$(NC)"

verify-add: build-release install ## Verify addition circuit
	@echo "$(BLUE)Verifying addition circuit...$(NC)"
	cd verification/scripts && \
		./compile_both.sh add && \
		./generate_witnesses.sh add && \
		python3 compare_results.py add

verify-subtract: build-release install ## Verify subtraction circuit
	@echo "$(BLUE)Verifying subtraction circuit...$(NC)"
	cd verification/scripts && \
		./compile_both.sh subtract && \
		./generate_witnesses.sh subtract && \
		python3 compare_results.py subtract

verify-all: build-release install ## Run full verification suite
	@echo "$(BLUE)Running full verification suite...$(NC)"
	cd verification/scripts && ./run_all_verification.sh
	@echo ""
	@echo "$(GREEN)Full verification complete!$(NC)"

verify: verify-add ## Run basic verification (default: add circuit)

# ============================================================================
# Comprehensive Testing
# ============================================================================

test-all: test-fast test-integration ## Run all tests except verification
	@echo ""
	@echo "$(GREEN)All tests passed!$(NC)"

ci: test-all ## Run all CI checks (fast + integration)
	@echo ""
	@echo "$(GREEN)All CI checks passed!$(NC)"

# ============================================================================
# Development Workflow
# ============================================================================

dev: format build test-unit ## Quick dev cycle: format, build, test
	@echo "$(GREEN)Development cycle complete!$(NC)"

pre-commit: format-check lint test-unit ## Pre-commit checks
	@echo "$(GREEN)Pre-commit checks passed!$(NC)"

pre-push: test-all ## Pre-push checks (all tests)
	@echo "$(GREEN)Ready to push!$(NC)"

# ============================================================================
# Examples
# ============================================================================

run-example: ## Run an example (usage: make run-example EXAMPLE=01_functions/functions)
	@if [ -z "$(EXAMPLE)" ]; then \
		echo "$(YELLOW)Usage: make run-example EXAMPLE=01_functions/functions$(NC)"; \
		exit 1; \
	fi
	@echo "$(BLUE)Running example: $(EXAMPLE)$(NC)"
	lof compile examples/$(EXAMPLE).lof

# ============================================================================
# Information
# ============================================================================

info: ## Show project information
	@echo "$(BLUE)Lof Project Information$(NC)"
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
