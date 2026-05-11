# Navegante CLI v0.1.0 Plan

## 1. Goals and scope

Ship `nvg`, a Rust CLI that talks to a new `/cli/v1` JSON API on `navegante_web`. v0.1.0 covers: device-flow auth, listing orgs/datacenters, creating and inspecting infrastructure projects (Hobby and paid via Stripe Checkout), and creating, listing, inspecting, and tailing logs for app deployments.

Out of scope for v0.1.0: scoped tokens (all tokens are scope `*`), team invites, billing management beyond the initial checkout, GitHub OAuth refresh management.

## 2. Routing and namespace

All CLI traffic uses a brand-new namespace, isolated from existing `/api`:

- `namespace :cli do`
  - `get "auth", to: "auth#show"` (HTML page where user approves a device code)
  - `post "auth/approve", to: "auth#approve"`
  - `namespace :v1 do` (JSON, bearer auth required except device endpoints)
    - `get  "viewer"` -- current user info
    - `resources :tokens, only: [:index, :create, :destroy]`
    - `post "device_authorizations"` (unauthenticated: start device flow)
    - `post "device_authorizations/poll"` (unauthenticated: exchange device_code for token)
    - `resources :organizations, only: [:index]`
    - `resources :datacenters, only: [:index]`
    - `resources :infrastructure_projects, only: [:index, :show, :create, :destroy]` do
      - `resources :app_configurations, only: [:index, :show]`
      - `resources :app_deployments, only: [:index, :show, :create]` do
        - `get :logs, on: :member`
      - `end`
    - `end`

Existing `/api/v1/*` routes are not touched.

## 3. Authentication

### 3.1 ApiToken model (new)
- `belongs_to :user`
- columns: `name:string`, `token_digest:string` (SHA256 of plaintext, unique index), `last_used_at:datetime`, `expires_at:datetime` (nullable), `scopes:jsonb` default `["*"]`, `revoked_at:datetime`
- plaintext shown once on creation, prefix `nvg_`
- expiration choices in UI: 30 / 60 / 90 days (default 90), custom date, no expiry

### 3.2 CliTokenAuthenticatable concern (new, separate from existing ApiTokenAuthenticatable)
- `before_action :authenticate_cli_token!`
- reads `Authorization: Bearer <token>`, hashes, looks up active (not revoked, not expired) `ApiToken`, sets `Current.user` and `Current.api_token`, updates `last_used_at` async
- 401 JSON `{ error: { code: "unauthorized", message: ... } }` on failure

### 3.3 Current model
- add `attribute :api_token`

### 3.4 Pundit
- `pundit_user` returns `Current.user` in `Cli::V1::BaseController`

### 3.5 Device flow (`nvg auth login`)
1. CLI POSTs to `/cli/v1/device_authorizations` -> returns `{ device_code, user_code, verification_uri, expires_in, interval }`
2. CLI prints `user_code` and opens `verification_uri` in browser
3. User signs in (if needed), sees `user_code`, names the token, picks expiration, clicks Approve
4. Approve creates an `ApiToken`, links it to the `DeviceAuthorization` row
5. CLI polls `POST /cli/v1/device_authorizations/poll` with `device_code` until it gets `{ token, expires_at, name }` or `authorization_pending` / `expired`

### 3.6 DeviceAuthorization model (new)
- `device_code` (random, unique), `user_code` (short human code), `expires_at`, `interval`, `approved_at`, `api_token_id` (nullable), `user_id` (nullable until approved)

## 4. Reused server logic

CLI controllers delegate to existing organizers; no business logic duplicated:
- `InfrastructureProjects::Organizers::{Index, Show, Create, Destroy, CreateStripeCheckout}`
- `AppDeployments::Organizers::{Index, Show, Create}`
- `NaveganteStripeProduct#create_checkout_session`

Both Hobby and paid plans go through Stripe Checkout. The create-project response always returns `{ project: {...}, checkout_url: "https://checkout.stripe.com/..." }`. CLI opens the URL and polls `GET /cli/v1/infrastructure_projects/:id` until status leaves `pending_payment`.

## 5. JSON shapes (v1)

- Errors: `{ "error": { "code": "string", "message": "string", "details": {} } }`
- Pagination: `{ "data": [...], "page": N, "per_page": N, "total": N }`
- Timestamps: ISO8601 UTC
- Deployment list item: `{ id, version_number, status, created_at, finished_at, commit_sha }` (commit message and description intentionally dropped)
- Deployment show: list fields + `app_configuration_id`, `triggered_by`, `image`, `error_message`
- Logs: `{ deployment_id, log_string, fetched_at }` (mirrors `Api::AppDeployments::LogsController`)

## 6. Rust crate layout

Rename package and binary to `nvg`. Single crate, modules per command group.

```
cli/
  Cargo.toml                # name = "nvg"
  src/
    main.rs                 # parse + dispatch
    cli.rs                  # clap derive tree
    config.rs               # ~/.config/nvg/config.toml read/write
    output.rs               # human + --json renderers, color via owo-colors
    api/
      mod.rs                # reqwest::Client wrapper, base_url, auth header
      error.rs              # ApiError enum, maps server error codes
      models.rs             # serde structs mirroring §5
      auth.rs               # device flow calls
      orgs.rs
      datacenters.rs
      projects.rs
      apps.rs
      deployments.rs
    commands/
      mod.rs
      auth.rs               # login, logout, status, tokens list/revoke
      orgs.rs               # list
      dcs.rs                # list
      projects.rs           # list, show, create, delete
      apps.rs               # list, show (per project)
      deployments.rs        # list, show, create, logs
```

Dependencies: `clap` (derive), `reqwest` (rustls, json), `tokio` (rt-multi-thread, macros), `serde`, `serde_json`, `toml`, `directories`, `owo-colors`, `anyhow`, `thiserror`, `webbrowser`, `indicatif` (spinner during polling).

## 7. Command tree

```
nvg auth login
nvg auth logout
nvg auth status
nvg auth tokens list
nvg auth tokens revoke <id>

nvg orgs list

nvg dcs list

nvg projects list [--org <id>]
nvg projects show <project_id>
nvg projects create --name <name> --datacenter <id> --plan <hobby|standard|...> [--org <id>]
nvg projects delete <project_id>

nvg projects <project_id> apps list
nvg projects <project_id> apps show <app_id>

nvg projects <project_id> apps <app_id> deployments list [--page N]
nvg projects <project_id> apps <app_id> deployments show <version>
nvg projects <project_id> apps <app_id> deployments create [--image <ref>] [--branch <name>] [--commit <sha>]
nvg projects <project_id> apps <app_id> deployments logs <version> [--follow]
```

Global flags: `--json`, `--no-color`, `--profile <name>`, `-v` / `-vv`.

No default-app shortcut; the full path is required.

## 8. Config file

`~/.config/nvg/config.toml`:
```
default_profile = "default"

[profiles.default]
api_url = "https://app.navegante.io"
token   = "nvg_..."
```

`nvg auth login` writes the token here. `nvg auth logout` deletes it (and calls `DELETE /cli/v1/tokens/:id` best-effort).

## 9. Error UX

- 401 -> `Error: not authenticated. Run \`nvg auth login\`.`
- 402 / `requires_payment` -> print message and the `checkout_url`, exit 2
- 403 -> `Error: not authorized for this resource.`
- 404 -> `Error: <resource> not found.`
- 422 -> print field errors line by line
- 5xx -> `Error: server error (<code>). Try again or contact support.`
- network -> `Error: could not reach <api_url>: <reason>.`

Exit codes: 0 ok, 1 generic error, 2 payment required, 3 auth required, 4 not found.

## 10. Web UI additions

- `account_hubs/api_tokens` index/new/destroy (ERB + ViewComponent, daisyUI)
  - new form: name, expiration radio (30/60/90/custom/never), default 90
  - create action shows plaintext token once with copy button and warning
- `cli/auth` page: shows pending device code, user fills in name + expiration, Approve / Deny buttons

## 11. Tests

Rails (Minitest + VCR where Stripe is involved):
- `ApiTokenTest`: digest, expiry, revocation, scope check
- `CliTokenAuthenticatableTest`: 401 paths, sets `Current.user`, updates `last_used_at`
- `Cli::V1::ViewerControllerTest`
- `Cli::V1::TokensControllerTest`
- `Cli::V1::DeviceAuthorizationsControllerTest` (start + poll states)
- `Cli::V1::InfrastructureProjectsControllerTest` (index/show/create returns checkout_url, destroy)
- `Cli::V1::AppDeploymentsControllerTest` (index/show/create/logs)
- `Cli::AuthControllerTest` (HTML approve flow)
- Policy tests reused

Rust:
- `api::error` mapping unit tests
- `output` snapshot tests for human and `--json` modes
- integration test against a `wiremock` server for each command

## 12. Implementation order (12 PRs)

1. ApiToken model + migration + `CliTokenAuthenticatable` concern + `Current.api_token` + tests
2. `account_hubs/api_tokens` CRUD UI (ERB + ViewComponent)
3. Rename `cli/` crate to `nvg`, scaffold modules, add deps, wire `clap` tree, stub commands
4. `Cli::V1::BaseController`, `viewer`, `tokens` endpoints + Rust `nvg auth status`, `tokens list/revoke`
5. `DeviceAuthorization` model + `cli/auth` HTML page + `device_authorizations` endpoints + Rust `nvg auth login/logout`
6. `organizations` and `datacenters` index endpoints + Rust `nvg orgs list`, `nvg dcs list`
7. `infrastructure_projects` index/show/destroy + Rust `nvg projects list/show/delete`
8. `infrastructure_projects` create with Stripe Checkout return + Rust `nvg projects create` (opens browser, polls)
9. `app_configurations` index/show + Rust `nvg projects <id> apps list/show`
10. `app_deployments` index/show + Rust `deployments list/show`
11. `app_deployments` create + Rust `deployments create`
12. `app_deployments` logs (with `--follow` poll loop) + Rust `deployments logs`

## 13. Open follow-ups (post v0.1.0)

- Scoped tokens (replace `["*"]`)
- `nvg gh auth` GitHub re-link command
- Team invite / membership commands
- Shell completions (`nvg completions <shell>`)
- Homebrew tap and prebuilt binaries
