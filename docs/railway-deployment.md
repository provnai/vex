# Deploying VEX on Railway

This guide will walk you through deploying the VEX Protocol API server on Railway in under 5 minutes.

## One-Click Deployment

The fastest way to get started is using the official ProvnAI VEX template:

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/template/vex-protocol?referralCode=provnai)

## Manual Setup

If you prefer to configure the deployment manually, follow these steps:

### 1. Create a New Project

1. Go to [Railway](https://railway.app/) and create a new project.
2. Select **Deploy from GitHub repo**.
3. Point it to `https://github.com/provnai/vex`.

### 2. Configure Environment Variables

The VEX server requires the following environment variables:

| Variable | Description | Required |
|----------|-------------|----------|
| `VEX_JWT_SECRET` | A secure 32-character string for JWT auth. | **Yes** |
| `OPENAI_API_KEY` | Your OpenAI API key. | Optional* |
| `ANTHROPIC_API_KEY` | Your Anthropic API key. | Optional* |
| `VEX_PORT` | Port the server binds to (default: `8080`). | No |
| `VEX_ENV` | Set to `railway` for optimized defaults. | No |
| `VEX_DEV_MODE` | Set to `1` to bypass hardware requirements (Default: `1`). | No |
| `VEX_HARDWARE_SEED` | 64-character hex seed for identity (Optional). | No |

*\*At least one LLM provider key is required for non-mock execution.*

### 3. Persistent Storage

VEX uses a SQLite database by default. To ensure your data persists across deployments:

1. In the Railway dashboard, go to **Settings** -> **Volumes**.
2. Click **Add Volume**.
3. Set the mount path to `/data`.
4. Ensure the `DATABASE_URL` environment variable is set to `sqlite:///data/vex.db?mode=rwc` (this is the default in the Dockerfile).

### 4. Verification

Once deployed, you can verify the installation by hitting the health endpoint:

```bash
curl https://your-vex-project.up.railway.app/health
```

You should receive a `200 OK` response:

```json
{
  "status": "healthy",
  "version": "0.2.0",
  "timestamp": "2026-03-02T..."
}
```

## Next Steps

- **Interactive API**: Explore the Swagger UI at `https://your-vex-project.up.railway.app/swagger-ui`.
- **Connect Agents**: Use the [VEX CLI](https://github.com/provnai/vex/tree/main/crates/vex-cli) to connect to your remote instance.
- **Join the Community**: Join the [ProvnAI Research Portal](https://provnai.com) to contribute to the cognitive safety mission.
