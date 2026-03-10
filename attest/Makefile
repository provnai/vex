RUST_DIR = attest-rs
LIB_NAME = attest_rs
BUILD_MODE ?= debug

ifeq ($(BUILD_MODE),release)
	CARGO_FLAGS = --release
	RUST_TARGET_DIR = $(RUST_DIR)/target/release
else
	CARGO_FLAGS =
	RUST_TARGET_DIR = $(RUST_DIR)/target/debug
endif

.PHONY: all build test lint clean help rust-build

# Default target
all: help

rust-build:
	@echo "Building Rust security core ($(BUILD_MODE))..."
	cd $(RUST_DIR) && cargo build $(CARGO_FLAGS)

build: rust-build
	@echo "Building attest CLI..."
	go build -o attest ./cmd/attest

test: rust-build
	@echo "Running tests..."
	go test -v -race ./...

clean:
	@echo "Cleaning artifacts..."
	rm -f attest coverage.txt coverage.html
	cd $(RUST_DIR) && cargo clean
	find . -name "*.test" -delete
	find . -name "*.out" -delete

install: rust-build
	@echo "Installing attest..."
	go install ./cmd/attest

# Release targets
release-dry-run:
	@echo "=== Release Dry Run ==="
	@echo "Version: $$(cat VERSION)"
	@echo "This would build all platforms and create artifacts."
	@echo ""
	@echo "Actions that WOULD be performed:"
	@echo "1. Build attest-linux-amd64"
	@echo "2. Build attest-linux-arm64"
	@echo "3. Build attest-darwin-amd64"
	@echo "4. Build attest-darwin-arm64"
	@echo "5. Build attest-windows-amd64.exe"
	@echo "6. Generate SHA256 checksums"
	@echo "7. Create release tag v$$$(cat VERSION)"
	@echo ""
	@echo "To perform actual release, run: make release"

release: release-dry-run
	@echo "=== Performing Release ==="
	@VERSION=$$(cat VERSION); \
	echo "Building v$$VERSION..."; \
	GOOS=linux GOARCH=amd64 go build -o attest-linux-amd64-$$VERSION ./cmd/attest; \
	GOOS=linux GOARCH=arm64 go build -o attest-linux-arm64-$$VERSION ./cmd/attest; \
	GOOS=darwin GOARCH=amd64 go build -o attest-darwin-amd64-$$VERSION ./cmd/attest; \
	GOOS=darwin GOARCH=arm64 go build -o attest-darwin-arm64-$$VERSION ./cmd/attest; \
	GOOS=windows GOARCH=amd64 go build -o attest-windows-amd64-$$VERSION.exe ./cmd/attest; \
	echo "Binaries built."
	@VERSION=$$(cat VERSION); \
	echo "Generating checksums..."; \
	sha256sum attest-linux-amd64-$$VERSION attest-linux-arm64-$$VERSION attest-darwin-amd64-$$VERSION attest-darwin-arm64-$$VERSION attest-windows-amd64-$$VERSION.exe > attest_$$VERSION_checksums.txt; \
	echo "Checksums written to attest_$$VERSION_checksums.txt"
	@echo ""
	@echo "=== Release Artifacts Ready ==="
	@echo "Files created:"
	@ls -la attest-*$$(cat VERSION)* 2>/dev/null || echo "No files found - run 'make build-all' first"
	@echo ""
	@echo "Next steps:"
	@echo "1. Review artifacts: ls attest-*"
	@echo "2. Verify checksums: sha256sum -c attest_$$(cat VERSION)_checksums.txt"
	@echo "3. Commit: git add -A && git commit -m 'Release v$$(cat VERSION)'"
	@echo "4. Tag: git tag -a v$$(cat VERSION) -m 'Release v$$(cat VERSION)'"
	@echo "5. Push: git push origin main --tags"
	@echo "6. Create GitHub release"

verify-release:
	@echo "=== Verifying Release Artifacts ==="
	@VERSION=$$(cat VERSION); \
	if [ ! -f "attest-linux-amd64-$$VERSION" ]; then \
		echo "ERROR: attest-linux-amd64-$$VERSION not found"; \
		exit 1; \
	fi
	@VERSION=$$(cat VERSION); \
	if [ ! -f "attest-linux-arm64-$$VERSION" ]; then \
		echo "ERROR: attest-linux-arm64-$$VERSION not found"; \
		exit 1; \
	fi
	@VERSION=$$(cat VERSION); \
	if [ ! -f "attest-darwin-amd64-$$VERSION" ]; then \
		echo "ERROR: attest-darwin-amd64-$$VERSION not found"; \
		exit 1; \
	fi
	@VERSION=$$(cat VERSION); \
	if [ ! -f "attest-darwin-arm64-$$VERSION" ]; then \
		echo "ERROR: attest-darwin-arm64-$$VERSION not found"; \
		exit 1; \
	fi
	@VERSION=$$(cat VERSION); \
	if [ ! -f "attest-windows-amd64-$$VERSION.exe" ]; then \
		echo "ERROR: attest-windows-amd64-$$VERSION.exe not found"; \
		exit 1; \
	fi
	@VERSION=$$(cat VERSION); \
	if [ ! -f "attest_$$VERSION_checksums.txt" ]; then \
		echo "ERROR: attest_$$VERSION_checksums.txt not found"; \
		exit 1; \
	fi
	@echo "All artifacts present."
	@echo ""
	@echo "Verifying checksums..."
	@VERSION=$$(cat VERSION); \
	sha256sum -c attest_$$VERSION_checksums.txt && echo "Checksums verified!"
	@echo ""
	@echo "Binary permissions check:"
	@VERSION=$$(cat VERSION); \
	for f in attest-linux-* attest-darwin-* attest-windows-*.exe; do \
		if [ -f "$$f" ]; then \
			perms=$$(stat -c "%a" "$$f" 2>/dev/null || stat -f "%OLp" "$$f" 2>/dev/null); \
			echo "  $$f: $$perms"; \
		fi; \
	done
	@echo ""
	@echo "=== Verification Complete ==="

help:
	@echo "Attest - Verifiable Agent Actions"
	@echo ""
	@echo "Targets:"
	@echo "  build          Build the attest binary"
	@echo "  test           Run tests with race detector"
	@echo "  test-coverage  Run tests and generate coverage report"
	@echo "  lint           Run golangci-lint"
	@echo "  lint-fix       Run linter with auto-fix"
	@echo "  clean          Clean build artifacts"
	@echo "  deps           Download dependencies"
	@echo "  deps-update    Update dependencies"
	@echo "  build-all      Build for all platforms"
	@echo "  install        Install attest to GOPATH"
	@echo "  release-dry-run Show what release would do (no changes)"
	@echo "  release        Build all release artifacts"
	@echo "  verify-release Verify release artifacts exist and checksums match"
	@echo "  help           Show this help message"
