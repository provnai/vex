# vex-protocol-cli

Command-line interface for VEX — verified AI agent tooling.

## Installation

```bash
cargo install vex-protocol-cli
```

## Commands

### Tools
- `vex tools list` — List available tools
- `vex tools run <name> '<json>'` — Execute tools

### Verification
- `vex verify --audit <file>` — Verify audit chain integrity
- `vex verify --db <file>` — Verify VEX database

### Info
- `vex info` — System information

## Built-in Tools

| Tool | Description |
|------|-------------|
| `calculator` | Math expressions |
| `datetime` | Time formatting |
| `uuid` | UUID v4 generation |
| `hash` | SHA-256/SHA-512 |
| `regex` | Pattern matching |
| `json_path` | JSON queries |

## Example

```bash
# List tools
vex tools list

# Run calculator
vex tools run calculator '{"expression": "2 + 2"}'

# Verify audit file
vex verify --audit session.json
```

## License

Apache-2.0 License - see [LICENSE](../../LICENSE) for details.
