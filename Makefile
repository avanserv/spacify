.DEFAULT_GOAL := help

# The CPython checkout the ruff round trip runs over. Its Lib/test is dropped: it holds files that
# are invalid Python on purpose.
CPYTHON_TAG := v3.14.8
CPYTHON_DIR := target/cpython

.PHONY: help install format lint test compat bench build hooks check clean

help: ## List available targets with their descriptions
	@grep -hE '^[a-zA-Z0-9_-]+:.*## ' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*## "} {printf "  \033[36m%-10s\033[0m %s\n", $$1, $$2}'

install: ## Install the git hooks
	prek install

format: ## Format the code (rustfmt)
	cargo fmt --all

lint: ## Lint the code (clippy) and audit the workflows (zizmor)
	cargo clippy --all-targets --locked -- -D warnings
	# Through the hook, so the version is the one .pre-commit-config.yaml pins.
	prek run zizmor --all-files

test: ## Run the test suite
	cargo test --locked

$(CPYTHON_DIR):
	git -c advice.detachedHead=false clone --quiet --depth 1 --branch $(CPYTHON_TAG) https://github.com/python/cpython.git $@
	rm -rf $@/Lib/test

compat: $(CPYTHON_DIR) ## Check that ruff and spacify agree, over the fixtures and CPython's Lib
	cargo build --release --locked
	scripts/ruff-compat.sh tests/fixtures
	scripts/ruff-compat.sh $(CPYTHON_DIR)/Lib

bench: $(CPYTHON_DIR) ## Time a check of CPython's Lib (hyperfine)
	cargo build --release --locked
	hyperfine --warmup 2 --ignore-failure 'target/release/spacify --check --quiet $(CPYTHON_DIR)/Lib'

build: ## Build the wheel and sdist into dist/ (maturin)
	rm -rf dist
	maturin build --release --out dist
	maturin sdist --out dist

hooks: ## Run every git hook over every tracked file (prek)
	prek run --all-files

# The non-mutating forms, so a red gate means the tree is wrong rather than that it was just
# rewritten: `make format` is the one that fixes.
check: ## Full gate: format, lint, tests, ruff round trip
	cargo fmt --all --check
	$(MAKE) --no-print-directory lint test compat

clean: ## Remove build output and the CPython checkout
	cargo clean
	rm -rf dist
