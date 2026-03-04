# Deploying VEX on Railway

Railway is the officially recommended cloud provider for deploying the VEX Protocol. This guide covers how to deploy VEX with its optimal cloud-native configuration, including PostgreSQL for high-performance persistence and OpenTelemetry for observability.

## Prerequisites

- A [Railway Account](https://railway.app/)
- An API Key from an LLM provider (e.g., DeepSeek, OpenAI, Anthropic, or Mistral)

## One-Click Deployment

Deploy the entire VEX stack, including a managed PostgreSQL instance, in one click:

[![Deploy on Railway](https://railway.com/button.svg)](https://railway.com/deploy/N9-iqS?referralCode=4AXmAG)

## Step-by-Step Configuration

If you are setting up the template manually or configuring it post-deployment, follow these best practices.

### 1. Database Configuration (PostgreSQL)
VEX uses SQLite by default but requires **PostgreSQL** for cloud-native scaling and high-concurrency environments.

1. Ensure the **PostgreSQL** service is added to your Railway project.
2. In the VEX service variables, add a reference to the private network database URL:
   - **Key**: `DATABASE_URL`
   - **Value**: `${{Postgres.DATABASE_URL}}`

*Note: Using the private URL (`DATABASE_URL`) rather than the public URL is crucial for security and eliminates egress bandwidth costs.*

### 2. Required Variables
You must set the following variables for VEX to start successfully:

- `VEX_JWT_SECRET`: A secure 32+ character string used for signing authentication tokens. In Railway, you can use the template variable generator to create a random string.
- `<PROVIDER>_API_KEY`: At least one API key for your chosen LLM (e.g., `DEEPSEEK_API_KEY`, `OPENAI_API_KEY`).

### 3. Rate Limiting (Optional)
VEX v0.3.0 introduced configurable Rate Limiting explicitly for cloud deployments.

- `VEX_LIMIT_FREE`: Requests per minute for basic tenants (default: 60)
- `VEX_LIMIT_STANDARD`: Requests per minute for standard tenants (default: 120)
- `VEX_LIMIT_PRO`: Requests per minute for pro tenants (default: 600)
- `VEX_DISABLE_RATE_LIMIT`: Set to `"true"` to completely bypass rate limiting (ideal if VEX is hosted behind an internal private network and another proxy).

### 4. OpenTelemetry Observability (Optional)
For enterprise auditing, VEX can stream its agent telemetry (execution times, tool decisions, LLM calls) directly to an OTLP-compatible collector.

- `OTEL_EXPORTER_OTLP_ENDPOINT`: e.g., `http://jaeger:4317` or a managed service private endpoint.
- `OTEL_SERVICE_NAME`: The identifier for this cluster, e.g., `vex-production-us-west`.

### 5. Inference & Adversarial Engine Configuration
VEX is not just a proxy; it's a reasoning engine. You can configure how it makes decisions and verifies data using the following variables:

- `VEX_DEFAULT_PROVIDER`: Select the primary LLM engine. Options: `deepseek`, `openai`, `anthropic`, `mistral`, `ollama`, or `mock`. (Default: `mock` if no keys are provided, `deepseek` if set).
- `VEX_DEFAULT_MODEL`: The specific model string to use (e.g., `deepseek-reasoner` or `gpt-4o`).
- `VEX_ADVERSARIAL`: Set to `"true"` to enable the built-in Red/Blue adversarial debate. When enabled, VEX will automatically spin up competing agent instances to verify tool outcomes through consensus before anchoring them to the database.
- `VEX_MAX_DEPTH`: The maximum chain-of-thought depth the agent is allowed to explore before forced termination (default: `5`).

## Advanced: McpVanguard (Edge-to-Cloud Topology)

If you are using **McpVanguard** to intercept and protect agent tool calls, you can configure Vanguard to use this Railway VEX deployment as its cryptographic Auditor. 

Simply point your local or private Vanguard `VEX_URL` to your VEX Railway public domain (e.g., `https://vex-production.up.railway.app`). VEX will handle all the PostgreSQL logging and Merkle-tree anchoring remotely, while Vanguard performs the fast local policy blocking.

## Advanced: Custom Code Development

VEX is built to be extremely modular. When you click the **Deploy on Railway** template button, Railway automatically clones the VEX repository into a new, private repository on your GitHub account.

Because you own the source code, you have the ultimate flexibility to bend VEX to your specific enterprise needs. For example, you can:
- **Add Custom Authentication:** Swap out the default JWT middleware for your company's existing SSO, Okta, or LDAP integration.
- **Inject Pre-flight Checks:** Add custom Rust logic to query your internal databases before VEX even evaluates an agent's request.
- **Tune the Adversarial Engine:** Modify the system prompts used by the Red/Blue team validators to check for specific compliance or regulatory rules unique to your industry.

Railway is already connected to your new repository. Any time you `git push` a custom change to your `main` branch, Railway automatically rebuilds the `Dockerfile` and deploys your customized version—zero CI/CD setup required.

## Health Checks and Volumes

Our Railway template is pre-configured with the following infrastructure defaults:
- **Healthchecks**: `/health` endpoint with a 300s timeout to allow initial DB migrations to complete.
- **Persistent Volume**: Mounted at `/data` automatically if you choose to fallback to SQLite. (If using Railway Postgres, the volume is safely ignored).
