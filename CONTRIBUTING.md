# Contributing to VEX

Thank you for your interest in contributing to VEX! This document provides guidelines for contributing to the project.

## 🎯 Project Vision

VEX (Verified Evolutionary Xenogenesis) is an open-source Rust framework for building adversarial, temporal, cryptographically-verified AI agents. We aim to make AI systems accountable and verifiable.

## 🤝 Ways to Contribute

- **Bug Reports** — Found a bug? Open an issue!
- **Feature Requests** — Have an idea? We'd love to hear it
- **Code Contributions** — Bug fixes, new features, performance improvements
- **Documentation** — Improve docs, add examples, fix typos
- **Testing** — Add test coverage, find edge cases

## 🚀 Getting Started

### Prerequisites

- **Rust 1.75+** (stable toolchain)
- **Git**
- **SQLite** (bundled via sqlx)

### Setup

```bash
# Clone the repository
git clone https://github.com/provnai/vex.git
cd vex

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run a demo (optional)
cargo run -p vex-demo
```

### Optional: LLM API Key

For testing with real LLMs:
```bash
export DEEPSEEK_API_KEY="sk-..."
```

## 📝 Development Workflow

### 1. Fork & Clone

```bash
# Fork the repo on GitHub, then:
git clone https://github.com/YOUR_USERNAME/vex.git
cd vex
git remote add upstream https://github.com/provnai/vex.git
```

### 2. Create a Branch

Use descriptive branch names:
```bash
git checkout -b feat/add-ollama-streaming
git checkout -b fix/merkle-tree-edge-case
git checkout -b docs/improve-api-examples
```

### 3. Make Changes

- Write clear, documented code
- Add tests for new functionality
- Update documentation as needed

### 4. Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add streaming support to Ollama provider
fix: handle empty Merkle tree edge case
docs: add examples for EpisodicMemory
test: add chaos tests for circuit breaker
refactor: simplify consensus voting logic
```

### 5. Submit a Pull Request

- Push your branch to your fork
- Open a PR against `main`
- Fill out the PR template
- Wait for CI to pass
- Request a review

## ✅ Code Standards

### Before Submitting

```bash
# Format code
cargo fmt --all

# Run clippy (must pass with no warnings)
cargo clippy --workspace --all-targets -- -D warnings

# Run tests
cargo test --workspace

# Build docs (check for warnings)
cargo doc --workspace --no-deps
```

### Style Guidelines

- Use `rustfmt` defaults
- Add doc comments (`///`) for all public items
- Include `# Examples` in doc comments where helpful
- Keep functions focused and under 50 lines when possible

## 🏗️ Crate-Specific Guidelines

| Crate | When to Modify |
|-------|----------------|
| `vex-core` | Agent structure, Merkle trees, context packets |
| `vex-adversarial` | Red/Blue verification, consensus protocols |
| `vex-temporal` | Memory management, decay strategies |
| `vex-llm` | Adding new LLM providers (OpenAI, Anthropic, etc.) |
| `vex-api` | HTTP endpoints, middleware, authentication |
| `vex-persist` | Storage backends (Redis, PostgreSQL, etc.) |
| `vex-queue` | Job processing, worker pool |
| `vex-runtime` | Agent orchestration, execution |
| `vex-anchor` | Blockchain anchoring backends (Solana, EIP-4844, etc.) |
| `vex-macros` | Procedural macros |

### Adding a New LLM Provider

1. Create `crates/vex-llm/src/your_provider.rs`
2. Implement the `LlmProvider` trait
3. Add to `crates/vex-llm/src/lib.rs` exports
4. Add tests in the same file
5. Update `README.md` with new provider

### Adding a Storage Backend

1. Create `crates/vex-persist/src/your_backend.rs`
2. Implement the `StorageBackend` trait
3. Add integration tests

## 🏷️ Issue Labels

| Label | Description |
|-------|-------------|
| `good-first-issue` | Great for newcomers |
| `help-wanted` | We need your help! |
| `bug` | Something isn't working |
| `enhancement` | New feature or improvement |
| `documentation` | Docs improvements |
| `performance` | Speed or memory optimization |

## 📊 Testing

### Unit Tests

```bash
cargo test --workspace
```

### Integration Tests

```bash
cargo test --workspace --test '*'
```

### With Real LLM (Ignored by Default)

```bash
DEEPSEEK_API_KEY="sk-..." cargo test -p vex-llm -- --ignored
```

### Benchmarks

```bash
cargo bench -p vex-core
```

## 📚 Documentation

- **API Docs**: https://provnai.dev
- **Architecture**: See [ARCHITECTURE.md](ARCHITECTURE.md)
- **Benchmarks**: See [BENCHMARKS.md](BENCHMARKS.md)

## 💬 Community

- **GitHub Discussions**: Ask questions, share ideas
- **Issues**: Bug reports and feature requests

## 📜 License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

Thank you for helping make VEX better! 🚀
