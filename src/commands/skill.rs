/// Print the nvg CLI skill / quick-reference guide.
pub fn run() {
    println!(
        r#"
╔══════════════════════════════════════════════════════════════════╗
║                    nvg CLI — Quick Reference                     ║
╚══════════════════════════════════════════════════════════════════╝

OVERVIEW
  nvg is the Navegante command-line tool for managing infrastructure,
  deployments, and apps from your terminal.

  Global flags (work with every command):
    --profile <NAME>   Use a named config profile  [default: default]
    --json             Machine-readable JSON output
    --no-color         Disable ANSI colour
    -v / -vv           Increase verbosity
    -h, --help         Print help for any command

AUTHENTICATION
  nvg auth login             Device-flow login (opens browser)
  nvg auth login --no-wait   Print URL/code and exit (no polling)
  nvg auth logout        Remove saved token
  nvg auth status        Show current user
  nvg auth tokens list   List API tokens
  nvg auth tokens revoke <ID>

ORGANIZATIONS & DATACENTERS
  nvg orgs list          List your organizations
  nvg dcs list           List available datacenters

PROJECTS
  nvg projects list [--org <ID>]
  nvg projects show <ID>
  nvg projects create \
    --name "my-app" --datacenter <ID> --plan <PLAN> --org <ID> \
    [--repo <GITHUB_REPO_ID>]...
  nvg projects delete <ID>

  Plans: hobby | hobby_plus | pro | pro_plus | enterprise
  Paid plans open Stripe Checkout in your browser automatically.

APPS  (nested under a project)
  nvg projects <PROJECT_ID> apps list
  nvg projects <PROJECT_ID> apps <APP_ID> show

DEPLOYMENTS  (nested under an app)
  nvg projects <PROJECT_ID> apps <APP_ID> deployments list [--page N]
  nvg projects <PROJECT_ID> apps <APP_ID> deployments show <VERSION>
  nvg projects <PROJECT_ID> apps <APP_ID> deployments create \
    [--branch <BRANCH>] [--commit <SHA>] [--image <IMAGE>]
  nvg projects <PROJECT_ID> apps <APP_ID> deployments logs <VERSION> \
    [--follow]

CONFIGURATION
  File: ~/.config/nvg/config.toml

  [default]
  api_base_url = "https://navegante.app"
  token        = "nvg_..."

  [staging]
  api_base_url = "https://staging.navegante.app"
  token        = "nvg_..."

  Switch profiles: nvg --profile staging ...
  Override config file: NVG_CONFIG=/path/to/config.toml nvg ...

QUICK-START
  1.  nvg auth login
  2.  nvg orgs list                       # note your org ID
  3.  nvg dcs list                         # note a datacenter ID
  4.  nvg projects create --name "hello" --datacenter 1 --plan hobby --org 1
  5.  nvg projects list
  6.  nvg projects 1 apps list
  7.  nvg projects 1 apps 1 deployments create --branch main

TIPS
  • All commands accept --json for scripting / piping to jq.
  • Run any command with -h for detailed flag descriptions.
  • Deployment logs stream in real-time with --follow.

Copyright (c) 2026 HOSTARI PHILIPPINES, INC.
"#
    );
}
