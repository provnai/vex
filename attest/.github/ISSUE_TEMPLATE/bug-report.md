---
name: Bug Report
description: Report a bug or unexpected behavior in Attest
title: "[Bug]: "
labels: ["bug", "triage"]
assignees: ""
---

## Description

<!-- Briefly describe the bug you encountered. What were you trying to do? -->

## Expected Behavior

<!-- What did you expect to happen? -->

## Actual Behavior

<!-- What actually happened? Include any error messages, stack traces, or unexpected output. -->

## Steps to Reproduce

<!-- Provide clear, numbered steps to reproduce the bug. Include minimal test cases if possible. -->

1. 
2. 
3. 

## Environment Details

<!-- The more information you provide, the faster we can diagnose and fix the issue. -->

- **Operating System**:
  - [ ] Windows (version: )
  - [ ] macOS (version: )
  - [ ] Linux (distribution: , version: )
  - [ ] Other: 

- **Go Version**:
  ```bash
  go version
  ```

- **Attest Version**:
  ```bash
  attest version
  ```

- **Python SDK Version** (if applicable):
  ```bash
  pip show attest-sdk
  ```

- **Node.js Version** (if applicable):
  ```bash
  node --version
  ```

- **Installation Method**:
  - [ ] Built from source (commit: )
  - [ ] Downloaded binary (version: )
  - [ ] pip install
  - [ ] npm install

## Logs and Output

<!-- Include relevant logs, error messages, or command output. Use code blocks for formatting. -->

```
<!-- Paste logs here -->
```

### Verbose Output

Run the command with verbose flags to get more detailed information:

```bash
# For CLI commands
attest --verbose <command>

# Or set environment variable
ATTEST_DEBUG=1 attest <command>
```

## Additional Context

<!-- Add any other context about the problem here. -->

### What Were You Trying To Accomplish?

<!-- Describe the task or workflow you were attempting when the bug occurred. -->

### Screenshots or Recordings

<!-- If applicable, add screenshots or screen recordings to help explain the problem. -->

### Related Issues

<!-- Link any related issues using GitHub's auto-linking or reference them here. -->

---

## Investigation Checklist

<!-- Help us understand what you've already tried. Check all that apply. -->

- [ ] I searched existing issues before reporting this bug
- [ ] I tested with the latest version of Attest
- [ ] I tested with a fresh configuration (`attest init`)
- [ ] I tested with different user/permission levels
- [ ] I tested on a different machine/environment
- [ ] I reviewed the documentation for related features
- [ ] I checked for known workarounds

## Possible Causes

<!-- If you have any theories about what might be causing the bug, share them here. -->

---

**Thank you for reporting this bug! Your help makes Attest better for everyone.**
