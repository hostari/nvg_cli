//! `nvg projects <project_id> apps <app_id> deployments {list,show,...}` commands.

use crate::api::error::ApiError;
use crate::api::models::AppDeployment;
use crate::commands::Context;
use crate::output;
use anyhow::{Context as _, Result};

pub async fn list(
    ctx: &Context,
    project_id: String,
    app_id: String,
    page: Option<u32>,
) -> Result<()> {
    require_auth(ctx)?;
    let pid = parse_id(&project_id, "project_id")?;
    let aid = parse_id(&app_id, "app_id")?;
    let result = ctx.api.deployments_list(pid, aid, page, None).await?;

    if ctx.json {
        return output::print_json(&result);
    }

    if result.data.is_empty() {
        println!("No deployments.");
        return Ok(());
    }

    println!(
        "{:<6}  {:<8}  {:<14}  {:<12}  {:<24}  {}",
        "ID", "VERSION", "STATUS", "COMMIT", "CREATED", "FINISHED"
    );
    for d in &result.data {
        println!(
            "{:<6}  {:<8}  {:<14}  {:<12}  {:<24}  {}",
            d.id,
            truncate(&d.version_number, 8),
            truncate(&d.status, 14),
            truncate(d.commit_sha.as_deref().unwrap_or("-"), 12),
            d.created_at.as_deref().unwrap_or("-"),
            d.finished_at.as_deref().unwrap_or("-"),
        );
    }
    println!();
    println!(
        "Page {} of {} (showing {}/{})",
        result.page,
        ((result.total as f64) / (result.per_page.max(1) as f64)).ceil() as u32,
        result.data.len(),
        result.total
    );
    Ok(())
}

pub async fn show(
    ctx: &Context,
    project_id: String,
    app_id: String,
    version: String,
) -> Result<()> {
    require_auth(ctx)?;
    let pid = parse_id(&project_id, "project_id")?;
    let aid = parse_id(&app_id, "app_id")?;
    let dep = ctx.api.deployments_show(pid, aid, &version).await?;

    if ctx.json {
        return output::print_json(&dep);
    }
    print_deployment(&dep);
    Ok(())
}

pub async fn create(
    ctx: &Context,
    project_id: String,
    app_id: String,
    image: Option<String>,
    branch: Option<String>,
    commit: Option<String>,
) -> Result<()> {
    require_auth(ctx)?;
    let pid = parse_id(&project_id, "project_id")?;
    let aid = parse_id(&app_id, "app_id")?;
    let dep = ctx
        .api
        .deployments_create(
            pid,
            aid,
            branch.as_deref(),
            commit.as_deref(),
            image.as_deref(),
        )
        .await?;

    if ctx.json {
        return output::print_json(&dep);
    }
    println!("Created deployment {} ({}).", dep.id, dep.version_number);
    print_deployment(&dep);
    Ok(())
}

pub async fn logs(
    ctx: &Context,
    project_id: String,
    app_id: String,
    version: String,
    follow: bool,
) -> Result<()> {
    require_auth(ctx)?;
    let pid = parse_id(&project_id, "project_id")?;
    let aid = parse_id(&app_id, "app_id")?;

    let initial = ctx.api.deployments_logs(pid, aid, &version).await?;

    if ctx.json {
        return output::print_json(&initial);
    }

    use std::io::Write;
    let stdout = std::io::stdout();
    {
        let mut handle = stdout.lock();
        handle.write_all(initial.log_string.as_bytes()).ok();
        handle.flush().ok();
    }

    if !follow {
        return Ok(());
    }

    if is_terminal_status(initial.status.as_deref()) {
        return Ok(());
    }

    let mut printed: usize = initial.log_string.len();
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let next = match ctx.api.deployments_logs(pid, aid, &version).await {
            Ok(l) => l,
            Err(_) => continue,
        };
        if next.log_string.len() > printed && next.log_string.starts_with(&initial.log_string[..printed.min(initial.log_string.len())]) {
            let new_part = &next.log_string[printed..];
            let mut handle = stdout.lock();
            handle.write_all(new_part.as_bytes()).ok();
            handle.flush().ok();
            printed = next.log_string.len();
        } else if next.log_string != initial.log_string && next.log_string.len() != printed {
            // Server replaced the log buffer (rotation or rebuild). Reprint
            // the diff against what we already printed by clearing and
            // emitting only the unseen tail.
            if next.log_string.len() > printed {
                let mut handle = stdout.lock();
                handle.write_all(next.log_string[printed..].as_bytes()).ok();
                handle.flush().ok();
                printed = next.log_string.len();
            }
        }
        if is_terminal_status(next.status.as_deref()) {
            break;
        }
    }
    Ok(())
}

fn is_terminal_status(status: Option<&str>) -> bool {
    matches!(
        status,
        Some("completed") | Some("error") | Some("canceled") | Some("cancelled") | Some("failed")
    )
}

fn print_deployment(d: &AppDeployment) {
    println!("ID:              {}", d.id);
    println!("Version:         {}", d.version_number);
    println!("Status:          {}", d.status);
    println!(
        "App config:      {}",
        d.app_configuration_id.map(|n| n.to_string()).unwrap_or_else(|| "-".into())
    );
    println!(
        "Commit:          {}",
        d.commit_sha.as_deref().unwrap_or("-")
    );
    println!(
        "Image:           {}",
        d.image.as_deref().unwrap_or("-")
    );
    println!(
        "Triggered by:    {}",
        d.triggered_by.as_deref().unwrap_or("-")
    );
    println!(
        "Created:         {}",
        d.created_at.as_deref().unwrap_or("-")
    );
    println!(
        "Finished:        {}",
        d.finished_at.as_deref().unwrap_or("-")
    );
    if let Some(err) = d.error_message.as_deref() {
        println!("Error:           {err}");
    }
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
