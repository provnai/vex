# provnai.dev Deployment Guide

Complete guide to deploy VEX documentation to **provnai.dev** using GitHub Pages.

---

## Prerequisites

- GitHub repository with VEX code pushed
- Domain `provnai.dev` registered
- Access to DNS settings

---

## Step 1: Push Code to GitHub

```bash
cd c:\Users\quint\Desktop\vex

# Add all new files
git add -A

# Commit
git commit -m "docs: add community docs and provnai.dev workflow"

# Push to main
git push origin main
```

---

## Step 2: Enable GitHub Pages

1. Go to your repository on GitHub
2. Navigate to **Settings** → **Pages**
3. Under "Build and deployment":
   - **Source**: Select "GitHub Actions"
4. The `docs.yml` workflow will automatically run

---

## Step 3: Configure Custom Domain (DNS)

Add these DNS records for `provnai.dev`:

### Option A: CNAME Record (Recommended)

```
Type: CNAME
Name: @ (or leave blank)
Value: provnai.github.io
TTL: 3600
```

### Option B: A Records (Alternative)

If CNAME on root doesn't work, use GitHub's IP addresses:

```
Type: A
Name: @
Value: 185.199.108.153
       185.199.109.153
       185.199.110.153
       185.199.111.153
TTL: 3600
```

### For www subdomain (Optional)

```
Type: CNAME
Name: www
Value: provnai.github.io
TTL: 3600
```

---

## Step 4: Configure GitHub for Custom Domain

1. Go to **Settings** → **Pages**
2. Under "Custom domain", enter: `provnai.dev`
3. Click **Save**
4. Wait for DNS check (may take up to 24 hours)
5. Enable **Enforce HTTPS** once available

---

## Step 5: Verify Deployment

```bash
# Check DNS propagation
nslookup provnai.dev

# Test the site
curl -I https://provnai.dev
```

Visit `https://provnai.dev` — you should see the Rustdoc documentation.

---

## Workflow Details

The `.github/workflows/docs.yml` does the following:

1. **Trigger**: Runs on push to `main`
2. **Build**: Runs `cargo doc --workspace --no-deps`
3. **Redirect**: Creates `index.html` that redirects to `vex_core/index.html`
4. **CNAME**: Adds `provnai.dev` CNAME file
5. **Deploy**: Uploads to GitHub Pages

---

## Updating Documentation

Documentation updates automatically when you push to `main`:

```bash
# Make changes to code or doc comments
git add -A
git commit -m "docs: update API documentation"
git push origin main

# Workflow runs automatically
# Check Actions tab for status
```

---

## Troubleshooting

### DNS Not Propagating

```bash
# Check current DNS
dig provnai.dev +short

# Use different DNS server
nslookup provnai.dev 8.8.8.8
```

### Build Failures

```bash
# Check Actions tab on GitHub
# Common issues:
# - Clippy warnings (run: cargo clippy --workspace)
# - Missing dependencies (run: cargo build)
```

### Custom Domain Not Working

1. Verify CNAME file exists in deployed branch
2. Check GitHub Pages settings
3. Clear browser cache
4. Wait for DNS propagation (up to 24h)

---

## URLs After Deployment

| URL | Content |
|-----|---------|
| `https://provnai.dev` | Redirects to vex_core |
| `https://provnai.dev/vex_core/` | vex-core docs |
| `https://provnai.dev/vex_adversarial/` | vex-adversarial docs |
| `https://provnai.dev/vex_temporal/` | vex-temporal docs |
| `https://provnai.dev/vex_llm/` | vex-llm docs |
| `https://provnai.dev/vex_api/` | vex-api docs |
