SHELL = /bin/bash
.SHELLFLAGS = -euo pipefail -c

CARGO   = cargo
MEMBERS = posvault_handler posvault_store posvault_crypto posvault_auth \
          posvault_sign posvault_sync posvault_query posvault

# Default package jika ingin menjalankan CI untuk satu package
PKG ?= posvault_handler

.PHONY: all
all: build

.PHONY: help
help:
	@echo "Usage: make <target> [PKG=<package>]"
	@echo ""
	@echo "Targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  %-18s %s\n", $$1, $$2}'

.PHONY: init-readmes
init-readmes:
	@for crate in $(MEMBERS); do \
		if [ -d "$$crate" ]; then \
			readme="$$crate/README.md"; \
			if [ ! -f "$$readme" ]; then \
				echo "Creating $$readme"; \
				echo "# $$crate\n\nPart of the posvault workspace.\n\nSee [README](../README.md) for full documentation." > "$$readme"; \
			else \
				echo "$$readme already exists"; \
			fi; \
		else \
			echo "ERROR: Folder $$crate not found"; \
			exit 1; \
		fi; \
	done

.PHONY: check-readmes
check-readmes:
	@missing=0; \
	for crate in $(MEMBERS); do \
		if [ ! -f "$$crate/README.md" ]; then \
			echo "MISSING: $$crate/README.md"; \
			missing=$$((missing + 1)); \
		fi; \
	done; \
	if [ $$missing -gt 0 ]; then \
		echo "ERROR: $$missing README files missing. Run 'make init-readmes'"; \
		exit 1; \
	else \
		echo "All READMEs present."; \
	fi

.PHONY: build
build:
	$(CARGO) build --workspace

.PHONY: release
release:
	$(CARGO) build --release --workspace

.PHONY: check
check:
	$(CARGO) check --workspace

.PHONY: test
test:
	$(CARGO) test --workspace

.PHONY: test-verbose
test-verbose:
	RUST_BACKTRACE=1 $(CARGO) test --workspace -- --nocapture

.PHONY: watch-test
watch-test:
	$(CARGO) watch -x 'test --workspace'

.PHONY: watch-build
watch-build:
	$(CARGO) watch -x 'check --workspace'

.PHONY: fmt
fmt:
	$(CARGO) fmt --all

.PHONY: fmt-check
fmt-check:
	$(CARGO) fmt --all -- --check

.PHONY: clippy
clippy:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

.PHONY: lint
lint: fmt clippy

.PHONY: ci
ci: fmt-check clippy test-verbose

.PHONY: clean
clean:
	$(CARGO) clean

.PHONY: doc
doc:
	$(CARGO) doc --workspace --no-deps

.PHONY: doc-open
doc-open: doc
	$(CARGO) doc --workspace --no-deps --open

.PHONY: bench
bench:
	$(CARGO) bench --workspace

.PHONY: update
update:
	$(CARGO) update

.PHONY: audit
audit:
	@if command -v cargo-audit >/dev/null 2>&1; then \
		$(CARGO) audit; \
	else \
		echo "cargo-audit not installed. Run: cargo install cargo-audit"; \
	fi

.PHONY: publish-check
publish-check: check-readmes
	@for crate in $(MEMBERS); do \
		echo "Packaging $$crate"; \
		$(CARGO) package -p "$$crate" --no-verify || exit 1; \
	done
	@echo "All crates are ready for publish."

.PHONY: publish-all
publish-all: check-readmes
	@echo "Publishing posvault_handler ..."
	$(CARGO) publish -p posvault_handler
	@sleep 5
	@echo "Publishing posvault_store ..."
	$(CARGO) publish -p posvault_store
	@sleep 5
	@echo "Publishing posvault_crypto ..."
	$(CARGO) publish -p posvault_crypto
	@sleep 5
	@echo "Publishing posvault_auth ..."
	$(CARGO) publish -p posvault_auth
	@sleep 5
	@echo "Publishing posvault_sign ..."
	$(CARGO) publish -p posvault_sign
	@sleep 5
	@echo "Publishing posvault_query ..."
	$(CARGO) publish -p posvault_query
	@sleep 5
	@echo "Publishing posvault_sync ..."
	$(CARGO) publish -p posvault_sync
	@sleep 5
	@echo "Publishing posvault (root) ..."
	$(CARGO) publish -p posvault
	@echo "All crates published successfully."

.PHONY: coverage
coverage:
	$(CARGO) llvm-cov --workspace --html
	@echo "Coverage report: target/llvm-cov/html/index.html"

.PHONY: version
version:
	@if [ -z "$(V)" ]; then \
		echo "Usage: make version V=<major|minor|patch|X.Y.Z>"; \
		exit 1; \
	fi
	@if [ ! -x "dev/version_bump.sh" ]; then \
		echo "ERROR: dev/version_bump.sh not found or not executable"; \
		exit 1; \
	fi
	dev/version_bump.sh $(V)

.PHONY: snap
snap:
	mkdir -p dev
	@for crate in $(MEMBERS); do \
		if [ -d "$$crate" ]; then \
			echo "Snapping $$crate/src"; \
			snapcat "$$crate/src" -f markdown -o "dev/$$crate.src.snapcat.md" || true; \
		fi; \
		if [ -d "$$crate/tests" ]; then \
			echo "Snapping $$crate/tests"; \
			snapcat "$$crate/tests" -f markdown -o "dev/$$crate.tests.snapcat.md" || true; \
		fi; \
	done
	@echo "Merging all snapshots into dev/root.md"
	cat dev/*.snapcat.md > dev/root.md 2>/dev/null || true
	@echo "Done. See dev/root.md"

.PHONY: run
run:
	$(CARGO) run

.PHONY: install
install:
	$(CARGO) install --path .

.PHONY: uninstall
uninstall:
	$(CARGO) uninstall posvault || true

.PHONY: rebuild
rebuild: release install

# ---------------------------------------------------------------------------
# Package-specific targets (pkg=<name>)
# ---------------------------------------------------------------------------
.PHONY: build-pkg
build-pkg:
	$(CARGO) build -p $(PKG)

.PHONY: release-pkg
release-pkg:
	$(CARGO) build --release -p $(PKG)

.PHONY: check-pkg
check-pkg:
	$(CARGO) check -p $(PKG)

.PHONY: test-pkg
test-pkg:
	$(CARGO) test -p $(PKG)

.PHONY: test-verbose-pkg
test-verbose-pkg:
	RUST_BACKTRACE=1 $(CARGO) test -p $(PKG) -- --nocapture

.PHONY: fmt-pkg
fmt-pkg:
	$(CARGO) fmt -p $(PKG)

.PHONY: fmt-check-pkg
fmt-check-pkg:
	$(CARGO) fmt -p $(PKG) -- --check

.PHONY: clippy-pkg
clippy-pkg:
	$(CARGO) clippy -p $(PKG) --all-targets --all-features -- -D warnings

.PHONY: ci-pkg
ci-pkg: fmt-check-pkg clippy-pkg test-verbose-pkg

.PHONY: doc-pkg
doc-pkg:
	$(CARGO) doc -p $(PKG) --no-deps

.PHONY: watch-test-pkg
watch-test-pkg:
	$(CARGO) watch -x 'test -p $(PKG)'

.PHONY: watch-build-pkg
watch-build-pkg:
	$(CARGO) watch -x 'check -p $(PKG)'

# ---------------------------------------------------------------------------
# Convenience aliases for common packages
# ---------------------------------------------------------------------------
.PHONY: handler
handler: PKG=posvault_handler
handler: ci-pkg

.PHONY: store
store: PKG=posvault_store
store: ci-pkg

.PHONY: crypto
crypto: PKG=posvault_crypto
crypto: ci-pkg

.PHONY: auth
auth: PKG=posvault_auth
auth: ci-pkg

.PHONY: sign
sign: PKG=posvault_sign
sign: ci-pkg

.PHONY: query
query: PKG=posvault_query
query: ci-pkg

.PHONY: sync
sync: PKG=posvault_sync
sync: ci-pkg

.PHONY: root-pkg
root-pkg: PKG=posvault
root-pkg: ci-pkg
