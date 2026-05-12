//! Command dispatch.

pub mod apps;
pub mod auth;
pub mod dcs;
pub mod deployments;
pub mod orgs;
pub mod projects;
pub mod skill;

use crate::api::ApiClient;
use crate::cli::{Cli, Command, ProjectsCommand};
use crate::config::{Config, ResolvedProfile};
use anyhow::Result;

/// Shared per-invocation context: resolved profile, API client, and global flags.
pub struct Context {
    pub profile: ResolvedProfile,
    pub api: ApiClient,
    pub json: bool,
}

impl Context {
    pub fn build(profile_override: Option<&str>, json: bool) -> Result<Self> {
        let cfg = Config::load()?;
        let profile = cfg.resolve(profile_override);
        let api = ApiClient::new(profile.api_url.clone(), profile.token.clone());
        Ok(Self { profile, api, json })
    }
}

pub async fn dispatch(args: Cli) -> Result<()> {
    // skill needs no auth or config
    if let Command::Skill = args.command {
        skill::run();
        return Ok(());
    }

    let ctx = Context::build(args.profile.as_deref(), args.json)?;

    match args.command {
        Command::Auth { command } => auth::run(command, &ctx).await,
        Command::Orgs { command } => orgs::run(command, &ctx).await,
        Command::Dcs { command } => dcs::run(command, &ctx).await,
        Command::Projects(p) => match p.command {
            ProjectsCommand::Nested(rest) => {
                let nested = crate::cli::parse_nested(&rest).map_err(|e| anyhow::anyhow!(e))?;
                projects::run_nested(nested, &ctx).await
            }
            other => projects::run(other, &ctx).await,
        },
        Command::Skill => unreachable!("handled above"),
    }
}
