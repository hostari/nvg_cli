use crate::api::auth::PollOutcome;
use crate::api::error::ApiError;
use crate::cli::{AuthCommand, TokensCommand};
use crate::commands::Context;
use crate::config::Config;
use crate::output;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub async fn run(cmd: AuthCommand, ctx: &Context) -> Result<()> {
    match cmd {
        AuthCommand::Login { no_wait } => login(ctx, no_wait).await,
        AuthCommand::Logout => logout(ctx).await,
        AuthCommand::Status => status(ctx).await,
        AuthCommand::Poll => poll(ctx).await,
        AuthCommand::Tokens { command } => match command {
            TokensCommand::List => tokens_list(ctx).await,
            TokensCommand::Revoke { id } => tokens_revoke(ctx, &id).await,
        },
    }
}

#[derive(Serialize)]
struct StatusReport<'a> {
    profile: &'a str,
    api_url: &'a str,
    authenticated: bool,
    viewer: Option<crate::api::models::Viewer>,
}

/// JSON state file saved by `login --no-wait` so `poll` can resume.
#[derive(Serialize, Deserialize)]
struct PendingAuth {
    device_code: String,
    expires_at: u64,
    interval: u64,
    verification_uri_complete: String,
    user_code: String,
    api_url: String,
}

fn pending_auth_path(profile: &str) -> Result<PathBuf> {
    use directories::ProjectDirs;
    let dirs = ProjectDirs::from("io", "Navegante", "nvg")
        .ok_or_else(|| anyhow::anyhow!("could not determine config directory"))?;
    Ok(dirs.config_dir().join(format!("pending_auth_{profile}.json")))
}

async fn login(ctx: &Context, no_wait: bool) -> Result<()> {
    let start = ctx.api.device_authorization_start().await?;

    if ctx.json {
        // Emit the start payload up-front so scripts can grab the user_code.
        output::print_json(&serde_json::json!({
            "verification_uri": start.verification_uri,
            "verification_uri_complete": start.verification_uri_complete,
            "user_code": start.user_code,
            "expires_in": start.expires_in,
            "interval": start.interval,
        }))?;
    } else {
        println!();
        println!("To authorize this CLI, visit:");
        println!("  {}", start.verification_uri.bold());
        println!("And enter the code:");
        println!("  {}", start.user_code.bold().green());
        println!();
        println!(
            "Or open the prefilled URL: {}",
            start.verification_uri_complete
        );
        println!();
    }

    if no_wait {
        // Save state so `nvg auth poll` can resume later.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let pending = PendingAuth {
            device_code: start.device_code.clone(),
            expires_at: now + start.expires_in,
            interval: start.interval,
            verification_uri_complete: start.verification_uri_complete.clone(),
            user_code: start.user_code.clone(),
            api_url: ctx.profile.api_url.clone(),
        };
        let path = pending_auth_path(&ctx.profile.name)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&pending)?;
        std::fs::write(&path, json)?;
        if !ctx.json {
            println!("State saved. Run `nvg auth poll` to complete sign-in.");
        }
        return Ok(());
    }

    // Best-effort browser open.
    let _ = webbrowser::open(&start.verification_uri_complete);

    let spinner = if ctx.json {
        None
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        pb.set_message("Waiting for approval...");
        pb.enable_steady_tick(Duration::from_millis(120));
        Some(pb)
    };

    let deadline = Instant::now() + Duration::from_secs(start.expires_in);
    let mut interval = Duration::from_secs(start.interval.max(1));

    let token_payload = loop {
        if Instant::now() >= deadline {
            if let Some(pb) = &spinner {
                pb.finish_and_clear();
            }
            return Err(anyhow::anyhow!(
                "device authorization expired before approval"
            ));
        }

        tokio::time::sleep(interval).await;

        match ctx.api.device_authorization_poll(&start.device_code).await {
            Ok(PollOutcome::Approved(tok)) => break tok,
            Ok(PollOutcome::Pending) => continue,
            Ok(PollOutcome::Expired) => {
                if let Some(pb) = &spinner {
                    pb.finish_and_clear();
                }
                return Err(anyhow::anyhow!(
                    "device authorization expired before approval"
                ));
            }
            Err(ApiError::Network(_)) => {
                // Transient network blip; back off slightly and keep trying.
                interval = (interval + Duration::from_secs(1)).min(Duration::from_secs(10));
                continue;
            }
            Err(e) => {
                if let Some(pb) = &spinner {
                    pb.finish_and_clear();
                }
                return Err(anyhow::Error::from(e));
            }
        }
    };

    if let Some(pb) = &spinner {
        pb.finish_and_clear();
    }

    let mut cfg = Config::load()?;
    cfg.set_profile(
        &ctx.profile.name,
        &ctx.profile.api_url,
        Some(token_payload.plaintext.clone()),
    );
    cfg.save()?;

    if ctx.json {
        output::print_json(&serde_json::json!({
            "authenticated": true,
            "profile": ctx.profile.name,
            "token": {
                "id": token_payload.id,
                "name": token_payload.name,
                "expires_at": token_payload.expires_at,
                "created_at": token_payload.created_at,
            },
        }))?;
    } else {
        println!("{}", "Authenticated.".green());
        println!("Profile: {}", ctx.profile.name);
        println!("Token:   {} (id {})", token_payload.name, token_payload.id);
    }
    Ok(())
}

async fn poll(ctx: &Context) -> Result<()> {
    let path = pending_auth_path(&ctx.profile.name)?;
    if !path.exists() {
        return Err(anyhow::anyhow!(
            "No pending auth found for profile '{}'. Run `nvg auth login --no-wait` first.",
            ctx.profile.name
        ));
    }

    let raw = std::fs::read_to_string(&path)?;
    let pending: PendingAuth = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("Could not parse pending auth state: {e}"))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if now >= pending.expires_at {
        let _ = std::fs::remove_file(&path);
        return Err(anyhow::anyhow!(
            "Pending device authorization has expired. Run `nvg auth login --no-wait` again."
        ));
    }

    if !ctx.json {
        println!();
        println!("Polling for approval of device flow...");
        println!(
            "  URL:  {}",
            pending.verification_uri_complete.bold()
        );
        println!("  Code: {}", pending.user_code.bold().green());
        println!();
    }

    let spinner = if ctx.json {
        None
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        pb.set_message("Waiting for approval...");
        pb.enable_steady_tick(Duration::from_millis(120));
        Some(pb)
    };

    let deadline = Instant::now() + Duration::from_secs(pending.expires_at.saturating_sub(now));
    let mut interval = Duration::from_secs(pending.interval.max(1));

    let token_payload = loop {
        if Instant::now() >= deadline {
            if let Some(pb) = &spinner {
                pb.finish_and_clear();
            }
            let _ = std::fs::remove_file(&path);
            return Err(anyhow::anyhow!(
                "Device authorization expired before approval."
            ));
        }

        tokio::time::sleep(interval).await;

        match ctx.api.device_authorization_poll(&pending.device_code).await {
            Ok(PollOutcome::Approved(tok)) => break tok,
            Ok(PollOutcome::Pending) => continue,
            Ok(PollOutcome::Expired) => {
                if let Some(pb) = &spinner {
                    pb.finish_and_clear();
                }
                let _ = std::fs::remove_file(&path);
                return Err(anyhow::anyhow!(
                    "Device authorization expired before approval."
                ));
            }
            Err(ApiError::Network(_)) => {
                interval = (interval + Duration::from_secs(1)).min(Duration::from_secs(10));
                continue;
            }
            Err(e) => {
                if let Some(pb) = &spinner {
                    pb.finish_and_clear();
                }
                return Err(anyhow::Error::from(e));
            }
        }
    };

    if let Some(pb) = &spinner {
        pb.finish_and_clear();
    }

    // Save token to config.
    let mut cfg = Config::load()?;
    cfg.set_profile(
        &ctx.profile.name,
        &ctx.profile.api_url,
        Some(token_payload.plaintext.clone()),
    );
    cfg.save()?;

    // Clean up state file.
    let _ = std::fs::remove_file(&path);

    if ctx.json {
        output::print_json(&serde_json::json!({
            "authenticated": true,
            "profile": ctx.profile.name,
            "token": {
                "id": token_payload.id,
                "name": token_payload.name,
                "expires_at": token_payload.expires_at,
                "created_at": token_payload.created_at,
            },
        }))?;
    } else {
        println!("{}", "Authenticated.".green());
        println!("Profile: {}", ctx.profile.name);
        println!("Token:   {} (id {})", token_payload.name, token_payload.id);
    }
    Ok(())
}

async fn logout(ctx: &Context) -> Result<()> {
    let mut revoked: Option<i64> = None;
    if ctx.profile.token.is_some() {
        if let Ok(tokens) = ctx.api.tokens_list().await {
            if let Some(current) = tokens.iter().find(|t| t.current) {
                let _ = ctx.api.tokens_revoke(&current.id.to_string()).await;
                revoked = Some(current.id);
            }
        }
    }

    let mut cfg = Config::load()?;
    cfg.clear_token(&ctx.profile.name);
    cfg.save()?;

    if ctx.json {
        output::print_json(&serde_json::json!({
            "logged_out": true,
            "profile": ctx.profile.name,
            "revoked_token_id": revoked,
        }))?;
    } else {
        match revoked {
            Some(id) => println!("Logged out (revoked token {id})."),
            None => println!("Logged out."),
        }
    }
    Ok(())
}

async fn status(ctx: &Context) -> Result<()> {
    if ctx.profile.token.is_none() {
        let report = StatusReport {
            profile: &ctx.profile.name,
            api_url: &ctx.profile.api_url,
            authenticated: false,
            viewer: None,
        };
        if ctx.json {
            output::print_json(&report)?;
        } else {
            println!("Profile:  {}", ctx.profile.name);
            println!("API URL:  {}", ctx.profile.api_url);
            println!(
                "Status:   {} (run `nvg auth login`)",
                "not authenticated".yellow()
            );
        }
        return Ok(());
    }

    match ctx.api.viewer().await {
        Ok(viewer) => {
            let report = StatusReport {
                profile: &ctx.profile.name,
                api_url: &ctx.profile.api_url,
                authenticated: true,
                viewer: Some(viewer.clone()),
            };
            if ctx.json {
                output::print_json(&report)?;
            } else {
                println!("Profile:  {}", ctx.profile.name);
                println!("API URL:  {}", ctx.profile.api_url);
                println!("User:     {} (id {})", viewer.email_address, viewer.id);
                println!("Status:   {}", "authenticated".green());
            }
            Ok(())
        }
        Err(e) => Err(anyhow::Error::from(e)),
    }
}

async fn tokens_list(ctx: &Context) -> Result<()> {
    require_auth(ctx)?;
    let tokens = ctx.api.tokens_list().await?;

    if ctx.json {
        return output::print_json(&tokens);
    }

    if tokens.is_empty() {
        println!("No active tokens.");
        return Ok(());
    }

    println!(
        "{:<6}  {:<24}  {:<25}  {:<25}  {}",
        "ID", "NAME", "CREATED", "LAST USED", "EXPIRES"
    );
    for t in tokens {
        let marker = if t.current { " *" } else { "  " };
        println!(
            "{:<6}{}{:<24}  {:<25}  {:<25}  {}",
            t.id,
            marker,
            truncate(&t.name, 24),
            t.created_at,
            t.last_used_at.unwrap_or_else(|| "never".into()),
            t.expires_at.unwrap_or_else(|| "never".into()),
        );
    }
    Ok(())
}

async fn tokens_revoke(ctx: &Context, id: &str) -> Result<()> {
    require_auth(ctx)?;
    ctx.api.tokens_revoke(id).await?;
    if !ctx.json {
        println!("Token {id} revoked.");
    } else {
        output::print_json(&serde_json::json!({ "revoked": id }))?;
    }
    Ok(())
}

fn require_auth(ctx: &Context) -> Result<()> {
    if ctx.profile.token.is_none() {
        return Err(anyhow::Error::from(ApiError::Unauthorized));
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

// Reserved for future use by main.rs to map ApiError into specific exit codes.
#[allow(dead_code)]
pub fn exit_code_for(err: &anyhow::Error) -> ExitCode {
    if let Some(api) = err.downcast_ref::<ApiError>() {
        return match api {
            ApiError::Unauthorized => ExitCode::from(3),
            ApiError::PaymentRequired(_) => ExitCode::from(2),
            ApiError::NotFound(_) => ExitCode::from(4),
            _ => ExitCode::from(1),
        };
    }
    ExitCode::from(1)
}
