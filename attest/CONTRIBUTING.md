# Contributing to Attest

Thank you for your interest in contributing to Attest! This comprehensive guide will help you get started with contributing to our project. We welcome contributions of all kinds, from bug fixes and new features to documentation improvements and example projects.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Environment Setup](#development-environment-setup)
- [Project Structure](#project-structure)
- [Coding Standards](#coding-standards)
- [Git Workflow](#git-workflow)
- [Testing Requirements](#testing-requirements)
- [Documentation Standards](#documentation-standards)
- [SDK Development](#sdk-development)
- [Pull Request Process](#pull-request-process)
- [Community](#community)

## Getting Started

### First-Time Contributors

If you're new to open source contribution, we recommend these resources:

- [GitHub's Open Source Guides](https://opensource.guide/how-to-contribute/)
- [First Contributions Repository](https://github.com/firstcontributions/first-contributions)
- [How to Contribute to Open Source](https://www.freecodecamp.org/news/how-to-contribute-to-open-source-projects/)

### Quick Start

```bash
# 1. Fork the repository on GitHub
# Visit https://github.com/provnai/attest and click "Fork"

# 2. Clone your fork locally
git clone https://github.com/YOURNAME/attest.git
cd attest

# 3. Add the upstream repository as a remote
git remote add upstream https://github.com/originalowner/attest.git

# 4. Create a feature branch
git checkout -b feature/your-feature-name

# 5. Make your changes and commit them
# (See Git Workflow below for commit message conventions)

# 6. Push to your fork
git push origin feature/your-feature-name

# 7. Create a Pull Request
# Visit https://github.com/originalowner/attest and create a PR
```

## Development Environment Setup

### Prerequisites

| Tool | Minimum Version | Recommended Version | Notes |
|------|-----------------|---------------------|-------|
| Go | 1.21 | 1.23.x | Primary language for core functionality |
| Git | 2.0 | Latest | Version control |
| Make | 3.0 | Latest | Build automation (optional) |
| SQLite | 3.0 | Latest | Bundled via mattn/go-sqlite3 |
| Node.js | 16.x | 20.x LTS | For JavaScript SDK development |
| Python | 3.9 | 3.11+ | For Python SDK development |

### Platform-Specific Setup

#### Windows

```powershell
# Install Go from https://go.dev/dl/
# Verify installation
go version

# Install Git from https://git-scm.com/download/win
# Or via Chocolatey: choco install git

# Clone and build
git clone https://github.com/provnai/attest.git
cd attest
go build -o attest.exe .\cmd\attest
.\attest.exe version

# Install golangci-lint (optional but recommended)
choco install golangci-lint
golangci-lint run ./...
```

#### macOS

```bash
# Install Homebrew if not installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Go
brew install go

# Install Git
brew install git

# Clone and build
git clone https://github.com/provnai/attest.git
cd attest
go build -o attest ./cmd/attest
./attest version

# Install golangci-lint
brew install golangci-lint/tap/golangci-lint
golangci-lint run ./...
```

#### Linux (Ubuntu/Debian)

```bash
# Install Go
wget https://go.dev/dl/go1.23.0.linux-amd64.tar.gz
sudo rm -rf /usr/local/go
sudo tar -C /usr/local -xzf go1.23.0.linux-amd64.tar.gz
export PATH=$PATH:/usr/local/go/bin

# Install Git
sudo apt update
sudo apt install git build-essential

# Clone and build
git clone https://github.com/provnai/attest.git
cd attest
go build -o attest ./cmd/attest
./attest version

# Install golangci-lint
curl -sSfL https://raw.githubusercontent.com/golangci/golangci-lint/master/install.sh | sh -s -- -b $(go env GOPATH)/bin v1.55.2
golangci-lint run ./...
```

### Verifying Your Setup

Run the following commands to verify your development environment:

```bash
# Check Go installation
go version
go env GOPATH GOROOT

# Run all tests
go test ./...

# Build the project
go build -o attest ./cmd/attest

# Run the CLI
./attest version

# Check code formatting
gofmt -d .

# Run linter (if installed)
golangci-lint run ./...
```

## Project Structure

```
attest/
├── cmd/                          # CLI command entry points
│   ├── attest/                   # Main CLI application
│   │   └── main.go
│   ├── root.go                   # Root command definition
│   ├── agent.go                  # Agent management commands
│   ├── attest.go                 # Attestation commands
│   ├── exec.go                   # Execution commands
│   ├── git.go                    # Git integration commands
│   ├── init.go                   # Initialization commands
│   ├── intent.go                 # Intent tracking commands
│   ├── policy.go                 # Policy management commands
│   ├── query.go                  # Query commands
│   └── verify.go                 # Verification commands
├── pkg/                          # Core packages
│   ├── attestation/              # Action signing and verification
│   │   └── attestation.go
│   ├── config/                   # Configuration management
│   │   └── config.go
│   ├── crypto/                   # Cryptographic operations
│   │   └── keys.go
│   ├── exec/                     # Reversible execution
│   │   └── executor.go
│   ├── identity/                 # Agent identity (AIDs)
│   │   └── agent.go
│   ├── intent/                   # Intent tracking
│   │   └── intent.go
│   ├── policy/                   # Policy engine
│   │   ├── defaults.go
│   │   ├── policy.go
│   │   └── yaml.go
│   └── storage/                  # SQLite database layer
│       └── db.go
├── internal/                     # Internal packages
│   └── test/                     # Test utilities and helpers
│       ├── git_hook_test.go
│       └── test_utils.go
├── sdk/                          # Language SDKs
│   ├── python/                   # Python SDK
│   │   ├── attest_client.py      # Main client implementation
│   │   ├── langchain_callback.py # LangChain integration
│   │   ├── setup.py              # Package setup
│   │   └── requirements.txt      # Dependencies
│   └── js/                       # JavaScript SDK
│       ├── attest-client.js      # Main client implementation
│       ├── index.js              # Entry point
│       ├── index.d.ts            # TypeScript definitions
│       ├── bin/cli.js            # CLI tool
│       └── package.json          # NPM package config
├── examples/                     # Example projects
│   ├── autogen-team/             # AutoGen multi-agent example
│   ├── crewai-research/          # CrewAI research example
│   └── langchain-chatbot/        # LangChain chatbot example
├── docs/                         # Documentation
├── scripts/                      # Build and utility scripts
├── .github/
│   ├── workflows/                # GitHub Actions
│   ├── ISSUE_TEMPLATE/           # Issue templates
│   └── pull_request_template.md  # PR template
├── Makefile                      # Build automation
├── go.mod                        # Go module definition
├── go.sum                        # Go dependency checksums
├── VERSION                       # Version file
├── CONTRIBUTING.md               # This file
├── README.md                     # Project readme
├── CODE_OF_CONDUCT.md            # Community guidelines
├── CHANGELOG.md                  # Release notes
└── SECURITY.md                   # Security policy
```

## Coding Standards

### Go Standards

The core Attest project is written in Go. All Go code must follow these standards:

#### Style Guidelines

```go
// 1. Use gofmt for formatting (automatic)
go fmt ./...

// 2. Follow Effective Go conventions
// https://golang.org/doc/effective_go

// 3. Use meaningful variable names
// Good:
agentID := generateAgentID()
attestation := createAttestation(action)

// Avoid:
a := generateA()
x := createX()

// 4. Group related code with blank lines
func ProcessAttestation(a *Attestation) error {
    // Validate input
    if err := a.Validate(); err != nil {
        return err
    }

    // Process attestation
    if err := a.Sign(); err != nil {
        return err
    }

    // Store result
    return db.Save(a)
}

// 5. Document all exported types and functions
// Policy represents a verification policy for agent actions.
type Policy struct {
    Name        string            `yaml:"name"`
    Rules       []PolicyRule      `yaml:"rules"`
    Description string            `yaml:"description"`
}
```

#### Required Go Practices

```go
// Error handling - use named returns for documentation
func ValidateAttestation(att *Attestation) (err error) {
    defer func() {
        if err != nil {
            log.Errorf("validation failed: %v", err)
        }
    }()

    // Validation logic
    return nil
}

// Context usage for cancellation
func (s *Storage) Query(ctx context.Context, query string) (*Result, error) {
    // Check for cancellation
    select {
    case <-ctx.Done():
        return nil, ctx.Err()
    default:
    }

    // Proceed with query
}

// Table-driven tests
func TestAttestation(t *testing.T) {
    tests := []struct {
        name    string
        att     *Attestation
        wantErr bool
    }{
        {
            name:    "valid attestation",
            att:     createValidAttestation(),
            wantErr: false,
        },
        {
            name:    "nil action",
            att:     createAttestationWithNilAction(),
            wantErr: true,
        },
    }

    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            err := ValidateAttestation(tt.att)
            if (err != nil) != tt.wantErr {
                t.Errorf("ValidateAttestation() error = %v, wantErr %v", err, tt.wantErr)
            }
        })
    }
}
```

#### Linting Configuration

We use golangci-lint. Create a `.golangci.yml` in the project root:

```yaml
run:
  timeout: 5m
  issues-exit-code: 1
  tests: true
  skip-dirs:
    - vendor
    - third_party

linters:
  enable-all: true
  disable:
    - dupl
    - lll
    - funlen
    - gochecknoglobals
    - gochecknoinits

linters-settings:
  gofmt:
    simplify: true
  goimports:
    local-prefixes: github.com/provnai/attest
  misspell:
    locale: US
  unconvert:
    bundle: true
```

### Python Standards

The Python SDK follows PEP 8 and includes type hints:

```python
"""Attest Python SDK Client."""

from __future__ import annotations

from typing import Any, Dict, List, Optional
from dataclasses import dataclass


@dataclass
class AttestationRequest:
    """Request object for creating attestations."""

    agent_id: str
    action: str
    intent_id: Optional[str] = None
    metadata: Optional[Dict[str, Any]] = None


class AttestClient:
    """Client for interacting with Attest service."""

    def __init__(self, api_key: str, base_url: str = "http://localhost:8080") -> None:
        """Initialize the Attest client.

        Args:
            api_key: API key for authentication.
            base_url: Base URL for the Attest API.
        """
        self.api_key = api_key
        self.base_url = base_url
        self._session = requests.Session()
        self._session.headers.update({"Authorization": f"Bearer {api_key}"})

    def create_attestation(self, request: AttestationRequest) -> Dict[str, Any]:
        """Create a new attestation.

        Args:
            request: The attestation request details.

        Returns:
            The created attestation data.

        Raises:
            AttestError: If the attestation creation fails.
        """
        try:
            response = self._session.post(
                f"{self.base_url}/v1/attestations",
                json=request.__dict__,
                timeout=30.0,
            )
            response.raise_for_status()
            return response.json()
        except requests.RequestException as e:
            raise AttestError(f"Failed to create attestation: {e}") from e
```

#### Python Linting

```bash
# Install development dependencies
pip install -r sdk/python/requirements-dev.txt

# Run linter
flake8 sdk/python/

# Run type checker
mypy sdk/python/

# Run tests
pytest sdk/python/ -v
```

### JavaScript Standards

The JavaScript SDK follows ESLint and includes TypeScript definitions:

```javascript
/**
 * Attest JavaScript Client
 * @packageDocumentation
 */

import { EventEmitter } from 'events';
import crypto from 'crypto';

/**
 * Client for interacting with Attest service
 */
export class AttestClient extends EventEmitter {
  /**
   * Create a new Attest client
   * @param {AttestClientOptions} options - Client configuration options
   */
  constructor(options) {
    super();

    this.apiKey = options.apiKey;
    this.baseUrl = options.baseUrl || 'http://localhost:8080';
    this.agentId = options.agentId;

    this.httpClient = createHttpClient({
      timeout: options.timeout || 30000,
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Content-Type': 'application/json',
      },
    });
  }

  /**
   * Create a new attestation
   * @param {AttestationRequest} request - Attestation request
   * @returns {Promise<AttestationResponse>}
   */
  async createAttestation(request) {
    const response = await this.httpClient.post(
      `${this.baseUrl}/v1/attestations`,
      request,
    );

    if (!response.success) {
      throw new AttestError(
        `Failed to create attestation: ${response.error}`,
      );
    }

    return response.data;
  }
}
```

#### JavaScript Linting

```bash
# Install dependencies
cd sdk/js
npm install

# Run linter
npm run lint

# Run type checker
npm run type-check

# Run tests
npm test
```

### Code Quality Requirements

| Metric | Minimum | Recommended |
|--------|---------|-------------|
| Test Coverage | 60% | 80% |
| Documentation | All exported symbols | All public symbols |
| Type Coverage | Go/Python: 100% | JS: 100% |
| Linting | Pass | Pass with warnings |

## Git Workflow

### Branch Naming Conventions

| Branch Type | Pattern | Example |
|-------------|---------|---------|
| Features | `feature/*` | `feature/add-reversible-execution` |
| Bug Fixes | `fix/*` | `fix/crypto-key-encoding` |
| Documentation | `docs/*` | `docs/update-api-reference` |
| Refactoring | `refactor/*` | `refactor/storage-layer` |
| Experiments | `experiment/*` | `experiment/new-policy-engine` |
| Releases | `release/*` | `release/v1.0.0` |

### Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

#### Types

| Type | Description |
|------|-------------|
| `feat` | A new feature |
| `fix` | A bug fix |
| `docs` | Documentation only changes |
| `style` | Changes that do not affect the meaning of the code (white-space, formatting, etc.) |
| `refactor` | A code change that neither fixes a bug nor adds a feature |
| `perf` | A code change that improves performance |
| `test` | Adding missing tests or correcting existing tests |
| `chore` | Changes to the build process or auxiliary tools |
| `ci` | Changes to CI configuration files and scripts |
| `build` | Changes that affect the build system or external dependencies |

#### Scopes

| Scope | Description |
|-------|-------------|
| `core` | Changes to core packages (pkg/) |
| `cli` | Changes to CLI commands (cmd/) |
| `sdk-py` | Changes to Python SDK |
| `sdk-js` | Changes to JavaScript SDK |
| `docs` | Changes to documentation |
| `examples` | Changes to example projects |
| `tests` | Changes to test files |
| `deps` | Dependency updates |

#### Examples

```
feat(core): Add support for reversible execution

Implement reversible execution for shell commands, allowing agents to
automatically undo dangerous operations.

Closes #123

fix(cli): Correct agent ID encoding issue

Fixed base64 encoding issue in agent ID generation that caused
verification failures on Windows.

BREAKING CHANGE: Agent IDs are now URL-safe encoded

docs(contributing): Update contribution guidelines

Added new section on SDK development standards.

refactor(storage): Simplify database query interface

Extracted common query patterns into reusable functions.
```

### Pull Request Process

1. **Create a descriptive PR title**
   - Use the same format as commit messages
   - Example: `feat(core): Add reversible execution support`

2. **Fill out the PR template completely**
   - Provide clear description of changes
   - List all testing performed
   - Check all requirements

3. **Keep PRs focused**
   - One feature or fix per PR
   - If you have multiple changes, create separate PRs

4. **Update documentation**
   - Update README if adding features
   - Add docstrings for new functions
   - Update API documentation

5. **Address review feedback**
   - Respond to all comments
   - Make requested changes
   - Re-request review after changes

6. **Squash and merge**
   - Maintainers will squash and merge
   - Keep commit history clean

## Testing Requirements

### Go Testing

```bash
# Run all tests
go test ./...

# Run tests with race detector
go test -race ./...

# Run tests with coverage
go test -coverprofile=coverage.txt ./...
go tool cover -html=coverage.txt

# Run specific package tests
go test ./pkg/crypto/...

# Run tests with verbose output
go test -v ./...

# Run tests matching pattern
go test -run TestAttestation ./...

# Benchmark tests
go test -bench=. -benchmem ./...
```

#### Test File Structure

```go
package attest_test

import (
    "testing"
    "github.com/provnai/attest/pkg/identity"
    "github.com/stretchr/testify/assert"
    "github.com/stretchr/testify/require"
)

func TestAgentIDGeneration(t *testing.T) {
    tests := []struct {
        name    string
        input   string
        wantErr bool
    }{
        {
            name:    "valid input",
            input:   "test-agent",
            wantErr: false,
        },
    }

    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            agent, err := identity.NewAgent(tt.input)

            if tt.wantErr {
                assert.Error(t, err)
                return
            }

            require.NoError(t, err)
            assert.NotEmpty(t, agent.ID)
            assert.Equal(t, tt.input, agent.Name)
        })
    }
}

func TestAgentVerification(t *testing.T) {
    agent := identity.NewAgent("test-agent")

    // Test signature verification
    signature := agent.Sign("test-data")
    valid := agent.Verify("test-data", signature)

    assert.True(t, valid, "signature should be valid")
}
```

### Python Testing

```bash
# Run all tests
pytest sdk/python/ -v

# Run with coverage
pytest sdk/python/ --cov=attest --cov-report=html

# Run specific test file
pytest sdk/python/test_attest_client.py -v

# Run test matching pattern
pytest sdk/python/ -k "test_create" -v
```

#### Pytest Structure

```python
import pytest
from attest import AttestClient, AttestationRequest


class TestAttestClient:
    """Test suite for AttestClient."""

    @pytest.fixture
    def client(self):
        """Create a test client."""
        return AttestClient(api_key="test-key", base_url="http://test.local")

    def test_create_attestation_success(self, client, requests_mock):
        """Test successful attestation creation."""
        requests_mock.post(
            "http://test.local/v1/attestations",
            json={"id": "att-123", "status": "verified"},
            status_code=201,
        )

        request = AttestationRequest(
            agent_id="agent-1",
            action="write_file",
            intent_id="intent-1",
        )

        response = client.create_attestation(request)

        assert response["id"] == "att-123"
        assert response["status"] == "verified"

    def test_create_attestation_failure(self, client, requests_mock):
        """Test attestation creation failure."""
        requests_mock.post(
            "http://test.local/v1/attestations",
            json={"error": "Invalid agent ID"},
            status_code=400,
        )

        request = AttestationRequest(
            agent_id="invalid-agent",
            action="write_file",
        )

        with pytest.raises(AttestError):
            client.create_attestation(request)
```

### JavaScript Testing

```bash
# Run all tests
npm test

# Run with coverage
npm test -- --coverage

# Run specific test file
npm test -- test/attest-client.test.js

# Run test matching pattern
npm test -- --testNamePattern="createAttestation"
```

#### Jest Structure

```javascript
/**
 * @jest-environment jsdom
 */

import { AttestClient } from '../attest-client';

describe('AttestClient', () => {
  let client;

  beforeEach(() => {
    client = new AttestClient({
      apiKey: 'test-key',
      baseUrl: 'http://test.local',
      agentId: 'agent-1',
    });
  });

  describe('createAttestation', () => {
    it('should create attestation successfully', async () => {
      fetchMock.mockResolvedValue(
        new Response(
          JSON.stringify({ id: 'att-123', status: 'verified' }),
          { status: 201 },
        ),
      );

      const request = {
        agentId: 'agent-1',
        action: 'write_file',
        intentId: 'intent-1',
      };

      const response = await client.createAttestation(request);

      expect(response.id).toBe('att-123');
      expect(response.status).toBe('verified');
    });

    it('should throw on API error', async () => {
      fetchMock.mockResolvedValue(
        new Response(
          JSON.stringify({ error: 'Invalid agent ID' }),
          { status: 400 },
        ),
      );

      const request = {
        agentId: 'invalid-agent',
        action: 'write_file',
      };

      await expect(client.createAttestation(request)).rejects.toThrow(
        AttestError,
      );
    });
  });
});
```

### Test Coverage Requirements

| Component | Minimum Coverage | Comments |
|-----------|-----------------|----------|
| Core packages (pkg/) | 70% | Critical paths must be covered |
| CLI commands (cmd/) | 60% | Main workflows |
| Python SDK | 80% | All public methods |
| JavaScript SDK | 80% | All public methods |

## Documentation Standards

### Types of Documentation

| Type | Location | Format |
|------|----------|--------|
| API Reference | `docs/api.md` | Markdown |
| CLI Reference | `docs/cli.md` | Markdown |
| Architecture | `docs/architecture.md` | Markdown |
| Examples | `examples/*/README.md` | Markdown |
| SDK Docs | `sdk/*/README.md` | Markdown |
| Code Docs | In-source | Docstrings/Go comments |

### Go Documentation Format

```go
// Policy represents a verification policy for agent actions.
//
// Policies define rules that agents must follow when performing actions.
// Each policy contains a set of rules that are evaluated against
// incoming attestations to determine if the action should be allowed.
//
// Example:
//
//	policy := &Policy{
//	    Name:        "safe-execution",
//	    Description: "Block dangerous shell commands",
//	    Rules: []PolicyRule{
//	        {Action: "exec", Pattern: "rm -rf /"},
//	        {Action: "exec", Pattern: "sudo"},
//	    },
//	}
type Policy struct {
    // Unique identifier for the policy
    ID string `json:"id" yaml:"id"`

    // Human-readable name
    Name string `json:"name" yaml:"name"`

    // Detailed description of the policy
    Description string `json:"description" yaml:"description"`

    // Rules that make up this policy
    Rules []PolicyRule `json:"rules" yaml:"rules"`
}

// Validate checks if the policy configuration is valid.
//
// Returns an error if:
//   - Name is empty
//   - No rules are defined
//   - Any rule has invalid configuration
func (p *Policy) Validate() error {
    if p.Name == "" {
        return errors.New("policy name is required")
    }

    if len(p.Rules) == 0 {
        return errors.New("at least one rule is required")
    }

    for i, rule := range p.Rules {
        if err := rule.Validate(); err != nil {
            return fmt.Errorf("rule %d: %w", i, err)
        }
    }

    return nil
}
```

### Python Documentation Format

```python
class PolicyEngine:
    """Policy evaluation engine for agent actions.

    The PolicyEngine is responsible for evaluating incoming attestations
    against configured policies to determine if an action should be allowed
    or blocked.

    Attributes:
        policies: Dictionary of registered policies by ID.
        cache_enabled: Whether to cache policy evaluation results.

    Example:
        >>> engine = PolicyEngine()
        >>> engine.load_policy(my_policy)
        >>> result = engine.evaluate(attestation)
        >>> print(result.allowed)
        True

    Raises:
        PolicyError: If policy evaluation fails.
    """

    def __init__(self, cache_enabled: bool = True) -> None:
        """Initialize the policy engine.

        Args:
            cache_enabled: Whether to enable result caching for
                improved performance on repeated evaluations.
        """
        self.policies: Dict[str, Policy] = {}
        self.cache_enabled = cache_enabled
        self._cache: Dict[str, EvaluationResult] = {}

    def evaluate(
        self,
        attestation: Attestation,
        context: Optional[EvaluationContext] = None,
    ) -> EvaluationResult:
        """Evaluate an attestation against all registered policies.

        Args:
            attestation: The attestation to evaluate.
            context: Optional evaluation context with additional data.

        Returns:
            An EvaluationResult containing the overall decision and
            details of each policy evaluation.

        Raises:
            PolicyError: If evaluation encounters an unexpected error.
        """
        cache_key = self._get_cache_key(attestation, context)

        if self.cache_enabled and cache_key in self._cache:
            return self._cache[cache_key]

        result = self._evaluate_internal(attestation, context)

        if self.cache_enabled:
            self._cache[cache_key] = result

        return result
```

### JavaScript Documentation Format

```javascript
/**
 * PolicyEngine - Policy evaluation engine for agent actions
 *
 * @description
 * The PolicyEngine is responsible for evaluating incoming attestations
 * against configured policies to determine if an action should be allowed
 * or blocked.
 *
 * @example
 * ```typescript
 * const engine = new PolicyEngine({ cacheEnabled: true });
 * engine.loadPolicy(myPolicy);
 *
 * const result = await engine.evaluate(attestation);
 * console.log(result.allowed); // true or false
 * ```
 *
 * @public
 */
export class PolicyEngine extends EventEmitter {
  /**
   * Create a new PolicyEngine instance
   * @param {PolicyEngineOptions} options - Configuration options
   */
  constructor(options = {}) {
    super();

    this.policies = new Map();
    this.cacheEnabled = options.cacheEnabled ?? true;
    this.cache = new Map();
  }

  /**
   * Load a policy into the engine
   * @param {Policy} policy - Policy to load
   * @throws {PolicyError} If policy validation fails
   */
  loadPolicy(policy) {
    if (!policy || typeof policy.validate !== 'function') {
      throw new PolicyError('Invalid policy: missing validate method');
    }

    const validation = policy.validate();
    if (!validation.valid) {
      throw new PolicyError(`Policy validation failed: ${validation.error}`);
    }

    this.policies.set(policy.id, policy);
    this.emit('policy:loaded', policy);
  }

  /**
   * Evaluate an attestation against all policies
   * @param {Attestation} attestation - Attestation to evaluate
   * @param {EvaluationContext} [context] - Optional evaluation context
   * @returns {Promise<EvaluationResult>}
   */
  async evaluate(attestation, context) {
    const cacheKey = this._getCacheKey(attestation, context);

    if (this.cacheEnabled && this.cache.has(cacheKey)) {
      return this.cache.get(cacheKey);
    }

    const result = await this._evaluateInternal(attestation, context);

    if (this.cacheEnabled) {
      this.cache.set(cacheKey, result);
    }

    return result;
  }
}
```

## SDK Development

### Python SDK Development

```bash
# Set up development environment
cd sdk/python
python -m venv venv
source venv/bin/activate  # Linux/macOS
# or
.\venv\Scripts\activate   # Windows

# Install dependencies
pip install -r requirements.txt
pip install -r requirements-dev.txt

# Install in development mode
pip install -e .

# Run tests
pytest -v

# Build distribution
python setup.py sdist bdist_wheel

# Upload to PyPI (maintainers only)
twine upload dist/*
```

### JavaScript SDK Development

```bash
# Set up development environment
cd sdk/js

# Install dependencies
npm install

# Run tests
npm test

# Run linter
npm run lint

# Run type checker
npm run type-check

# Build for distribution
npm run build

# Publish to NPM (maintainers only)
npm publish
```

### Adding New SDK Features

1. Implement the feature in the Go core
2. Add Python SDK wrapper in `sdk/python/attest_client.py`
3. Add JavaScript SDK wrapper in `sdk/js/attest-client.js`
4. Update TypeScript definitions in `sdk/js/index.d.ts`
5. Add tests for all SDKs
6. Update documentation

## Pull Request Process

### Before Submitting

- [ ] Code follows all coding standards
- [ ] All tests pass (`go test ./...`, `pytest`, `npm test`)
- [ ] Linter passes (`golangci-lint run ./...`)
- [ ] Documentation updated
- [ ] Commit messages follow conventions
- [ ] PR description is complete

### PR Template

```markdown
## Description

Brief description of what this PR changes or adds.

## Type of Change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update
- [ ] Refactoring (no functional changes)

## Testing

Describe the testing you performed:

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed
- [ ] Test coverage maintained

## Checklist

- [ ] My code follows the project's style guidelines
- [ ] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
- [ ] Any dependent changes have been merged and published
```

### After Submitting

1. **Automated Checks**
   - CI pipeline runs
   - All tests must pass
   - Linter must pass
   - Coverage must be maintained

2. **Code Review**
   - Maintainers will review your PR
   - Address feedback promptly
   - Re-request review after changes

3. **Merge**
   - Maintainers squash and merge
   - PR is linked to relevant issues
   - Changes are included in next release

## Community

### Communication Channels

- **GitHub Issues**: Bug reports, feature requests
- **GitHub Discussions**: Questions, ideas, community chat
- **Discord**: Real-time discussion (link in README)

### Recognition

Contributors are recognized in:
- [CONTRIBUTORS.md](CONTRIBUTORS.md)
- Release notes
- Project documentation

### Code of Conduct

All contributors must follow our [Code of Conduct](CODE_OF_CONDUCT.md). Please report violations to [INSERT CONTACT METHOD].

## Additional Resources

- [Architecture Documentation](docs/architecture.md)
- [API Reference](docs/api.md)
- [CLI Reference](docs/cli.md)
- [Examples](examples/)
- [Master Task List](MASTER_TASK_LIST.md)

---

**Thank you for contributing to Attest!**
