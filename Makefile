.PHONY: all build release check test test-verbose fmt fmt-check clippy lint clean run install uninstall ci rebuild snap doc doc-open bench update audit publish-check publish-all version coverage watch-test watch-build help

MEMBERS = posvault_handler posvault_store posvault_crypto posvault_auth posvault_sign posvault_sync
SNAPCAT    = snapcat
SNAPCAT_OPTS =
CARGO      = cargo
RUSTC      = rustc
NIGHTLY    = nightly

all: build

help: 
	@printf "Usage:\n"
	@printf "  make <target>\n\n"
	@printf "Targets:\n"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

build: 
	$(CARGO) build

release: 
	$(CARGO) build --release

check: 
	$(CARGO) check --workspace

test: 
	$(CARGO) test --workspace

test-verbose: 
	$(CARGO) test --workspace -- --nocapture

watch-test: 
	$(CARGO) watch -x 'test --workspace'

watch-build: 
	$(CARGO) watch -x 'check --workspace'

fmt: 
	$(CARGO) fmt --all

fmt-check: 
	$(CARGO) fmt --all -- --check

clippy: 
	$(CARGO) clippy --all-targets --all-features -- -D warnings

lint: fmt clippy 

ci: fmt-check clippy test 

clean: 
	$(CARGO) clean

run: 
	$(CARGO) run

install: 
	$(CARGO) install --path .

uninstall: 
	$(CARGO) uninstall jsscli

rebuild: release install 

snap: 
	mkdir -p dev
	@for dir in $(MEMBERS); do \
		if [ -d "$$dir" ]; then \
			echo "📸 $$dir"; \
			$(SNAPCAT) $$dir -f markdown $(SNAPCAT_OPTS) -o dev/$$dir.src.snapcat.md; \
		fi; \
		if [ -d "$$dir/tests" ]; then \
			echo "📸 $$dir/tests"; \
			$(SNAPCAT) $$dir/tests -f markdown $(SNAPCAT_OPTS) -o dev/$$dir.tests.snapcat.md; \
		fi; \
	done
	@echo "Merging all snapshots into dev/root.md"
	cat dev/*.snapcat.md > dev/root.md
	@echo "Done. See dev/root.md"

doc: 
	$(CARGO) doc --workspace --no-deps

doc-open: doc 
	$(CARGO) doc --workspace --no-deps --open

bench: 
	$(CARGO) bench --workspace

update: 
	$(CARGO) update

audit: 
	@if command -v cargo-audit >/dev/null 2>&1; then \
		$(CARGO) audit; \
	else \
		echo "cargo-audit not installed. Run: cargo install cargo-audit"; \
	fi

publish-check: 
	@for member in $(MEMBERS); do \
		echo "👉 Packaging $$member"; \
		$(CARGO) package -p $$member --no-verify || exit 1; \
	done

publish-all:
	cargo publish -p libage_auth_handler
	sleep 10
	cargo publish -p libage_crypto
	sleep 10
	cargo publish -p libage_otp
	sleep 10
	cargo publish -p libage_authenticator
	sleep 10
	cargo publish -p age_auth

version: 
	@if [ -z "$(V)" ]; then \
		echo "Usage: make version V=<major|minor|patch|X.Y.Z>"; \
		exit 1; \
	fi
	dev/version_bump.sh $(V)

coverage: 
	$(CARGO) llvm-cov --workspace --html
	@echo "Coverage report written to target/llvm-cov/html/index.html"
