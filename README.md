# Navegante CLI (nvg)

Command-line interface for managing Navegante infrastructure and deployments.

## Features

- Device flow authentication (OAuth 2.0 RFC 8628)
- API token management
- Organization and datacenter listings
- Infrastructure project CRUD with Stripe Checkout support
- App configuration inspection
- Deployment creation, listing, showing, and log tailing

## Prerequisites

Before using the CLI, you must complete the following steps on the Navegante web platform:

1. **Sign up** at navegante.io (or your Navegante instance)
2. **Connect your GitHub account** via OAuth (required for repository access)
3. **Add a payment method** on Stripe (required for paid plans)

The CLI uses the Navegante web platform's existing user account, GitHub OAuth tokens, and Stripe customer data. You cannot create projects or deploy apps without completing these setup steps in the web UI first.

## Installation

### Build from source

Requires Rust 1.70 or later.

```bash
# Clone the repository
git clone https://github.com/hostari/nvg_cli.git
cd nvg_cli

# Build the release binary
cargo build --release

# The binary will be at ./target/release/nvg
# Optionally, copy it to your PATH:
cp ./target/release/nvg /usr/local/bin/
```

### Development build

```bash
cargo build
./target/debug/nvg --help
```

## Quick Start

### 1. Authenticate

The CLI uses device flow authentication. Run:

```bash
nvg auth login
```

This will:
1. Display a user code and open your browser
2. Prompt you to approve the device on the website
3. Poll for approval and save the token locally

Your token is stored in `~/.config/nvg/config.toml` by default.

### 2. Verify authentication

```bash
nvg auth whoami
```

### 3. List your organizations

```bash
nvg orgs list
```

### 4. Create a project

```bash
# List available datacenters
nvg datacenters list

# Create a hobby project (free, no Stripe checkout)
nvg projects create \
  --name "my-app" \
  --datacenter 1 \
  --plan hobby \
  --org 123

# Create a paid project (opens Stripe Checkout in browser)
nvg projects create \
  --name "my-production-app" \
  --datacenter 1 \
  --plan pro \
  --org 123 \
  --repo 456 \
  --repo 789
```

Available plans: `hobby`, `hobby_plus`, `pro`, `pro_plus`, `enterprise`

The CLI will:
- Create the project
- Open Stripe Checkout in your browser (for paid plans)
- Poll until payment succeeds or times out

### 5. Manage deployments

```bash
# List apps in a project
nvg projects 1 apps list

# Show app details
nvg projects 1 apps 2 show

# List deployments for an app
nvg projects 1 apps 2 deployments list

# Create a new deployment
nvg projects 1 apps 2 deployments create \
  --branch main \
  --commit abc123

# Show deployment details
nvg projects 1 apps 2 deployments show v3

# Tail deployment logs (follows until completion)
nvg projects 1 apps 2 deployments logs v3 --follow
```

## Usage

```
nvg [OPTIONS] <COMMAND>

Commands:
  auth          Authentication commands
  tokens        API token management
  orgs          List organizations
  datacenters   List datacenters
  projects      Manage infrastructure projects

Options:
      --profile <PROFILE>  Config profile to use [default: default]
      --json               Output JSON instead of human-readable format
  -h, --help               Print help
  -V, --version            Print version
```

### Authentication commands

```bash
nvg auth login       # Start device flow authentication
nvg auth logout      # Remove saved token
nvg auth whoami      # Show current user
```

### API token commands

```bash
nvg tokens list           # List your API tokens
nvg tokens create <NAME>  # Create a new token
nvg tokens revoke <ID>    # Revoke a token
```

### Project commands

```bash
nvg projects list                  # List all projects
nvg projects show <ID>             # Show project details
nvg projects create [OPTIONS]      # Create a new project
nvg projects delete <ID>           # Delete a project
```

### Nested project commands

```bash
# Apps
nvg projects <ID> apps list
nvg projects <ID> apps <APP_ID> show

# Deployments
nvg projects <ID> apps <APP_ID> deployments list [--page N]
nvg projects <ID> apps <APP_ID> deployments show <VERSION>
nvg projects <ID> apps <APP_ID> deployments create [OPTIONS]
nvg projects <ID> apps <APP_ID> deployments logs <VERSION> [--follow]
```

## Configuration

Config file location: `~/.config/nvg/config.toml`

Example:

```toml
[default]
api_base_url = "https://navegante.io"
token = "nvg_..."

[staging]
api_base_url = "https://staging.navegante.io"
token = "nvg_..."
```

Use `--profile staging` to switch profiles.

## JSON Output

All commands support `--json` for machine-readable output:

```bash
nvg --json projects list
nvg --json projects 1 apps 2 deployments show v3
```

## Development

```bash
# Run tests (when added)
cargo test

# Run with custom config
NVG_CONFIG=/tmp/test.toml cargo run -- auth whoami

# Format code
cargo fmt

# Lint
cargo clippy
```

## Limitations

- No scoped tokens yet (all tokens have `*` scope)
- No team invite management via CLI
- No billing management beyond initial Stripe Checkout
- GitHub token refresh is handled automatically by the web platform

See `cli-plan.md` for the full v0.1.0 specification and implementation roadmap.

## License

See [LICENSE](LICENSE) file.
