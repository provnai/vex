# vex-llm

LLM provider integrations for the VEX Protocol.

## Supported Providers

- **OpenAI** - GPT-4, GPT-3.5, etc.
- **Ollama** - Local LLM inference
- **DeepSeek** - DeepSeek models
- **Mistral** - Mistral AI models
- **Mock** - Testing provider

## Installation

```toml
[dependencies]
vex-llm = "0.1"

# With OpenAI support
vex-llm = { version = "0.1", features = ["openai"] }
```

## Quick Start

```rust
use vex_llm::{LlmProvider, OllamaProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = OllamaProvider::new("http://localhost:11434");
    let response = provider.complete("Hello, world!").await?;
    println!("{}", response);
    Ok(())
}
```

## License

MIT License - see [LICENSE](../../LICENSE) for details.
