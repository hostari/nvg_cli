# `@hostari/nvg`

Standalone TypeScript client and CLI for Navegante MicroApps. It implements the `/cli/v1` contract and coexists with the original Rust CLI in this repository.

## Requirements

- Node.js 20 or later
- A Navegante API token

## Install

```bash
npm install @hostari/nvg
```

For repository development:

```bash
cd typescript
npm install
npm run check
npm link
```

The CLI reads the existing Rust CLI configuration at `~/.config/nvg/config.toml` (or `$NVG_CONFIG`):

```toml
default_profile = "default"

[profiles.default]
api_url = "https://navegante.app"
token = "nvg_..."
```

`--api-url` and `--token` override the profile for automation.

## CLI

```bash
nvg micro-apps list [--org 12]
nvg micro-apps show 42
nvg micro-apps create --org 12 --name Docs --slug docs
nvg micro-apps publish 42 ./site --manifest micro-app.json --wait

nvg micro-apps publications list 42
nvg micro-apps publications show 42 7
nvg micro-apps publications finalize 42 7

# Secrets are accepted from stdin, not command-line arguments.
printf '%s' '{"API_URL":"https://example.test","TOKEN":"secret"}' | \
  nvg micro-apps environment set 42 7
```

Use global `--json` for a stable JSON result on stdout. Status progress is written to stderr.

A manifest example:

```json
{
  "version": 1,
  "mode": "static",
  "publish_dir": "dist",
  "required_env": ["API_URL"]
}
```

A directory source is packaged as deterministic `tar.gz`. Existing `.tar.gz` archives are uploaded unchanged. The client computes the Base64 MD5 required by Active Storage and the lowercase SHA-256 required by publication finalization.

## Library

```ts
import { MicroAppClient } from "@hostari/nvg";

const client = new MicroAppClient({
  baseUrl: "https://navegante.app",
  token: process.env.NVG_TOKEN!,
});

const app = await client.microApps.create({
  organizationId: 12,
  name: "Docs",
  slug: "docs",
});

const publication = await client.publish({
  microAppId: app.micro_app_id,
  source: "./site",
  manifest: { version: 1, mode: "static", publish_dir: "dist" },
  wait: true,
});
```

The storage upload receives only the headers returned by the upload session. The Navegante bearer token is never forwarded to storage. API failures throw `NvgApiError`, preserving `status`, machine-readable `code`, and safe `details`.
