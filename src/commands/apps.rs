//! `nvg projects <project_id> apps {list,show}` commands.

use crate::api::error::ApiError;
use crate::api::models::AppConfiguration;
use crate::commands::Context;
use crate::output;
use anyhow::{Context as _, Result};

pub async fn list(ctx: &Context, project_id: &str) -> Result<()> {
    require_auth(ctx)?;
    let pid = parse_id(project_id, "project_id")?;
    let apps = ctx.api.apps_list(pid).await?;

    if ctx.json {
        return output::print_json(&apps);
    }

    if apps.is_empty() {
        println!("No app configurations.");
        return Ok(());
    }

    println!(
        "{:<6}  {:<32}  {:<24}  {:<14}  {:<14}",
        "ID", "REPO", "DOMAIN", "DEPLOYMENT", "ACCESS"
    );
    for a in apps {
        println!(
            "{:<6}  {:<32}  {:<24}  {:<14}  {:<14}",
            a.id,
            truncate(a.repository_full_name.as_deref().unwrap_or("-"), 32),
            truncate(a.fallback_domain_name.as_deref().unwrap_or("-"), 24),
            truncate(a.deployment_status.as_deref().unwrap_or("-"), 14),
            truncate(a.accessibility_status.as_deref().unwrap_or("-"), 14),
        );
    }
    Ok(())
}

pub async fn show(ctx: &Context, project_id: &str, app_id: &str) -> Result<()> {
    require_auth(ctx)?;
    let pid = parse_id(project_id, "project_id")?;
    let aid = parse_id(app_id, "app_id")?;
    let app = ctx.api.apps_show(pid, aid).await?;

    if ctx.json {
        return output::print_json(&app);
    }
    print_app(&app);
    Ok(())
}

fn print_app(a: &AppConfiguration) {
    println!("ID:                   {}", a.id);
    println!("Project:              {}", a.infrastructure_project_id);
    println!(
        "Repository:           {}",
        a.repository_full_name.as_deref().unwrap_or("-")
    );
    println!(
        "Domain:               {}",
        a.fallback_domain_name.as_deref().unwrap_or("-")
    );
    println!(
        "IP:                   {}",
        a.ip_address.as_deref().unwrap_or("-")
    );
    println!(
        "Deployment status:    {}",
        a.deployment_status.as_deref().unwrap_or("-")
    );
    println!(
        "Accessibility status: {}",
        a.accessibility_status.as_deref().unwrap_or("-")
    );
    println!(
        "Flavor:               {}",
        a.deployment_flavor.as_deref().unwrap_or("-")
    );
    println!(
        "Status:               {}",
        a.status.as_deref().unwrap_or("-")
    );
    println!(
        "Created:              {}",
        a.created_at.as_deref().unwrap_or("-")
    );
}

fn parse_id(s: &str, label: &str) -> Result<i64> {
    s.parse::<i64>()
        .with_context(|| format!("{label} must be a numeric id (got {s:?})"))
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

fn require_auth(ctx: &Context) -> Result<()> {
    if ctx.profile.token.is_none() {
        return Err(anyhow::Error::from(ApiError::Unauthorized));
    }
    Ok(())
}
