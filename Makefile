.DEFAULT_GOAL := help

# The CPython checkout the ruff round trip runs over, one directory per tag so a bump re-clones.
CPYTHON_TAG := v3.14.8
CPYTHON_DIR := target/cpython-$(CPYTHON_TAG)
# The interpreter scripts/ast-equal.py parses with: it must accept everything in CPYTHON_TAG's Lib.
export COMPAT_PYTHON := 3.14

# Files the round trip cannot use. A CPython bump that adds another one fails `make compat`, which
# is the news: decide whether it belongs here.
# Invalid on purpose, or not UTF-8.
CPYTHON_SKIP := \
	test/tokenizedata/badsyntax_pep3120.py \
	test/tokenizedata/badsyntax_3131.py \
	test/tokenizedata/bad_coding2.py \
	test/encoded_modules/module_koi8_r.py \
	test/encoded_modules/module_iso_8859_1.py
# Valid Python that tree-sitter-python rejects: README.md lists the constructs under "Known limits".
CPYTHON_SKIP += test/test_annotationlib.py test/test_type_params.py test/test_compile.py
# Ruff 0.16.9 needs two passes to settle a `for` target here, with or without spacify.
CPYTHON_SKIP += test/test_unicodedata.py

# The Rust projects the rustfmt round trip runs over: stable code, one edition each, since
# tree-sitter-rust refuses some of the nightly syntax in the standard library.
RIPGREP_TAG := 15.2.0
RIPGREP_DIR := target/ripgrep-$(RIPGREP_TAG)
TOKIO_TAG := tokio-1.54.1
TOKIO_DIR := target/tokio-$(TOKIO_TAG)

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
	cd $@/Lib && rm $(CPYTHON_SKIP)

$(RIPGREP_DIR):
	git -c advice.detachedHead=false clone --quiet --depth 1 --branch $(RIPGREP_TAG) https://github.com/BurntSushi/ripgrep.git $@

$(TOKIO_DIR):
	git -c advice.detachedHead=false clone --quiet --depth 1 --branch $(TOKIO_TAG) https://github.com/tokio-rs/tokio.git $@

compat: $(CPYTHON_DIR) $(RIPGREP_DIR) $(TOKIO_DIR) ## Check that spacify agrees with ruff and rustfmt, over the fixtures and real projects
	cargo build --release --locked --bin spacify --example tokens_equal
	scripts/ruff-compat.sh tests/fixtures
	scripts/ruff-compat.sh $(CPYTHON_DIR)/Lib
	scripts/rustfmt-compat.sh tests/fixtures 2024
	scripts/rustfmt-compat.sh $(RIPGREP_DIR) 2024
	scripts/rustfmt-compat.sh $(TOKIO_DIR) 2021

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
check: ## Full gate: format, lint, tests, formatter round trips
	cargo fmt --all --check
	$(MAKE) --no-print-directory lint test compat

clean: ## Remove build output and the CPython checkout
	cargo clean
	rm -rf dist
