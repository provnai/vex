## Description

<!-- Describe your changes in detail. What did you change and why? -->

## Type of Change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update (changes to documentation only)
- [ ] Refactoring (no functional changes, no API changes)
- [ ] Tests (adding or updating tests)
- [ ] Chore (maintenance tasks, build tools, CI/CD)

## Related Issues

<!-- Link related issues using GitHub's auto-linking or closing keywords -->

Fixes # (issue number)
Related to # (issue or PR number)

## Testing Performed

<!-- Describe the testing you performed to verify your changes -->

### Manual Testing

- [ ] Tested locally with `go build` and `./attest`
- [ ] Tested specific command:
  ```bash
  attest <command> --flags
  ```

### Automated Tests

- [ ] All existing tests pass
  ```bash
  go test ./...
  ```

- [ ] New tests added
  ```bash
  go test -v ./pkg/... -run TestNewFeature
  ```

- [ ] Tests with race detector
  ```bash
  go test -race ./...
  ```

- [ ] Coverage maintained or improved
  ```bash
  go test -coverprofile=coverage.txt ./...
  ```

### SDK Testing

- [ ] Python SDK tested
  ```bash
  cd sdk/python && pytest
  ```

- [ ] JavaScript SDK tested
  ```bash
  cd sdk/js && npm test
  ```

## Checklist

<!-- Ensure all requirements are met before submitting -->

### General

- [ ] My code follows the project's coding standards
- [ ] I have read and understood the [CONTRIBUTING.md](CONTRIBUTING.md) guide
- [ ] My changes generate no new compiler warnings
- [ ] My changes generate no new linting errors

### Code Quality

- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] I have added or updated docstrings for new functions
- [ ] I have updated README if adding new features

### Testing

- [ ] I have added tests that prove my fix is effective or my feature works
- [ ] New and existing unit tests pass locally with my changes
- [ ] I have tested edge cases and error conditions
- [ ] I have verified that existing functionality is not broken

### Git

- [ ] My commit messages follow the [Conventional Commits](https://www.conventionalcommits.org/) format
- [ ] My branch is up to date with the main branch
- [ ] I have squashed my commits into logical units (if needed)

### Security

- [ ] My changes do not introduce security vulnerabilities
- [ ] I have not hardcoded any secrets or credentials
- [ ] Sensitive data is properly handled and logged as `[REDACTED]`

### Reviewers

- [ ] I have requested review from @maintainers
- [ ] I am available to answer questions about my changes
- [ ] I understand that changes may require additional iterations

---

## Additional Notes

<!-- Add any additional information that reviewers should know -->

### Breaking Changes

<!-- Describe any breaking changes and how to migrate -->

### Migration Steps

<!-- If this PR includes breaking changes, describe migration steps -->

### Screenshots

<!-- If your changes affect the UI or CLI output, add screenshots -->

### Related PRs

<!-- Link any related pull requests -->

---

## Release Notes

<!-- For feature PRs, describe what will appear in the changelog -->

```markdown
## [Unreleased]

### Added
- Description of new feature

### Changed
- Description of change

### Fixed
- Description of fix

### Security
- Security-related changes
```

---

## Submitting Your PR

1. Ensure all checklist items are complete
2. Request review from maintainers
3. Respond to feedback and make necessary changes
4. Once approved, a maintainer will merge your PR

**Thank you for contributing to Attest!**
