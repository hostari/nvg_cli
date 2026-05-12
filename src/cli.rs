use clap::{Parser, Subcommand};

/// Navegante CLI
#[derive(Parser, Debug)]
#[command(name = "nvg", version, about = "Navegante CLI", long_about = None)]
pub struct Cli {
    /// Output JSON instead of human-readable text
    #[arg(long, global = true)]
    pub json: bool,

    /// Disable ANSI color in output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Config profile to use
    #[arg(long, global = true)]
    pub profile: Option<String>,

    /// Verbosity (-v, -vv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Authentication commands
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// List organizations
    Orgs {
        #[command(subcommand)]
        command: OrgsCommand,
    },
    /// List datacenters
    Dcs {
        #[command(subcommand)]
        command: DcsCommand,
    },
    /// Infrastructure project commands
    Projects(ProjectsArgs),
    /// Show the nvg CLI skill / quick-reference guide
    Skill,
}

#[derive(Subcommand, Debug)]
pub enum AuthCommand {
    /// Log in via device flow
    Login,
    /// Log out and revoke the local token
    Logout,
    /// Show current auth status
    Status,
    /// Manage API tokens
    Tokens {
        #[command(subcommand)]
        command: TokensCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum TokensCommand {
    /// List API tokens
    List,
    /// Revoke an API token
    Revoke {
        /// Token ID
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum OrgsCommand {
    /// List organizations
    List,
}

#[derive(Subcommand, Debug)]
pub enum DcsCommand {
    /// List datacenters
    List,
}

#[derive(Parser, Debug)]
pub struct ProjectsArgs {
    #[command(subcommand)]
    pub command: ProjectsCommand,
}

#[derive(Subcommand, Debug)]
pub enum ProjectsCommand {
    /// List infrastructure projects
    List {
        /// Filter by organization ID
        #[arg(long)]
        org: Option<String>,
    },
    /// Show an infrastructure project
    Show {
        /// Project ID
        project_id: String,
    },
    /// Create an infrastructure project
    Create {
        /// Project name
        #[arg(long)]
        name: String,
        /// Datacenter ID
        #[arg(long)]
        datacenter: String,
        /// Plan slug (hobby, hobby_plus, pro, pro_plus, enterprise)
        #[arg(long)]
        plan: String,
        /// Organization ID (required)
        #[arg(long)]
        org: String,
        /// GitHub repository ID to attach (repeat for multiple)
        #[arg(long = "repo")]
        repos: Vec<String>,
    },
    /// Delete an infrastructure project
    Delete {
        /// Project ID
        project_id: String,
    },
    /// Per-project app and deployment commands
    #[command(external_subcommand)]
    Nested(Vec<String>),
}

/// Parsed nested form: `nvg projects <project_id> apps ...`
#[derive(Debug)]
pub enum NestedProjectCommand {
    AppsList {
        project_id: String,
    },
    AppsShow {
        project_id: String,
        app_id: String,
    },
    DeploymentsList {
        project_id: String,
        app_id: String,
        page: Option<u32>,
    },
    DeploymentsShow {
        project_id: String,
        app_id: String,
        version: String,
    },
    DeploymentsCreate {
        project_id: String,
        app_id: String,
        image: Option<String>,
        branch: Option<String>,
        commit: Option<String>,
    },
    DeploymentsLogs {
        project_id: String,
        app_id: String,
        version: String,
        follow: bool,
    },
}

/// Parse the trailing positional args after `nvg projects <project_id> ...`
/// Supports the command shapes documented in cli-plan.md §7.
pub fn parse_nested(args: &[String]) -> Result<NestedProjectCommand, String> {
    let mut iter = args.iter().map(String::as_str);
    let project_id = iter.next().ok_or("missing <project_id>")?.to_string();
    let group = iter.next().ok_or("expected `apps` after project id")?;
    if group != "apps" {
        return Err(format!("unknown subcommand `{group}`, expected `apps`"));
    }
    let action = iter.next().ok_or("expected apps subcommand")?;
    match action {
        "list" => Ok(NestedProjectCommand::AppsList { project_id }),
        "show" => {
            let app_id = iter.next().ok_or("missing <app_id>")?.to_string();
            Ok(NestedProjectCommand::AppsShow { project_id, app_id })
        }
        _ => {
            // `nvg projects <pid> apps <app_id> deployments ...`
            let app_id = action.to_string();
            let group2 = iter.next().ok_or("expected `deployments` after app id")?;
            if group2 != "deployments" {
                return Err(format!("unknown group `{group2}`, expected `deployments`"));
            }
            let dep_action = iter.next().ok_or("expected deployments subcommand")?;
            let rest: Vec<&str> = iter.collect();
            match dep_action {
                "list" => {
                    let mut page = None;
                    let mut i = 0;
                    while i < rest.len() {
                        if rest[i] == "--page" {
                            page = rest.get(i + 1).and_then(|s| s.parse().ok());
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    Ok(NestedProjectCommand::DeploymentsList {
                        project_id,
                        app_id,
                        page,
                    })
                }
                "show" => {
                    let version = rest.first().ok_or("missing <version>")?.to_string();
                    Ok(NestedProjectCommand::DeploymentsShow {
                        project_id,
                        app_id,
                        version,
                    })
                }
                "create" => {
                    let mut image = None;
                    let mut branch = None;
                    let mut commit = None;
                    let mut i = 0;
                    while i < rest.len() {
                        match rest[i] {
                            "--image" => {
                                image = rest.get(i + 1).map(|s| s.to_string());
                                i += 2;
                            }
                            "--branch" => {
                                branch = rest.get(i + 1).map(|s| s.to_string());
                                i += 2;
                            }
                            "--commit" => {
                                commit = rest.get(i + 1).map(|s| s.to_string());
                                i += 2;
                            }
                            _ => i += 1,
                        }
                    }
                    Ok(NestedProjectCommand::DeploymentsCreate {
                        project_id,
                        app_id,
                        image,
                        branch,
                        commit,
                    })
                }
                "logs" => {
                    let version = rest.first().ok_or("missing <version>")?.to_string();
                    let follow = rest.iter().any(|s| *s == "--follow");
                    Ok(NestedProjectCommand::DeploymentsLogs {
                        project_id,
                        app_id,
                        version,
                        follow,
                    })
                }
                other => Err(format!("unknown deployments subcommand `{other}`")),
            }
        }
    }
}
