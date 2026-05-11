use crate::api::error::ApiError;
use crate::api::models::InfrastructureProject;
use crate::cli::{NestedProjectCommand, ProjectsCommand};
use crate::commands::Context;
use crate::output;
use anyhow::{Context as _, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::{Duration, Instant};

pub async fn run(cmd: ProjectsCommand, ctx: &Context) -> Result<()> {
    match cmd {
        ProjectsCommand::List { org } => list(ctx, org).await,
        ProjectsCommand::Show { project_id } => show(ctx, &project_id).await,
        ProjectsCommand::Create { name, datacenter, plan, org, repos } => {
            create(ctx, name, datacenter, plan, org, repos).await
        }
        ProjectsCommand::Delete { project_id } => delete(ctx, &project_id).await,
        ProjectsCommand::Nested(_) => unreachable!("handled in dispatch"),
    }
}

pub async fn run_nested(cmd: NestedProjectCommand, ctx: &Context) -> Result<()> {
    use NestedProjectCommand::*;
    match cmd {
        AppsList { project_id } => {
            crate::commands::apps::list(ctx, &project_id).await?;
        }
        AppsShow { project_id, app_id } => {
            crate::commands::apps::show(ctx, &project_id, &app_id).await?;
        }
        DeploymentsList { project_id, app_id, page } => {
            crate::commands::deployments::list(ctx, project_id, app_id, page).await?;
        }
        DeploymentsShow { project_id, app_id, version } => {
            crate::commands::deployments::show(ctx, project_id, app_id, version).await?;
        }
        DeploymentsCreate { project_id, app_id, image, branch, commit } => {
            crate::commands::deployments::create(ctx, project_id, app_id, image, branch, commit).await?;
        }
        DeploymentsLogs { project_id, app_id, version, follow } => {
            crate::commands::deployments::logs(ctx, project_id, app_id, version, follow).await?;
        }
    }
    Ok(())
}

async fn list(ctx: &Context, org: Option<String>) -> Result<()> {
    require_auth(ctx)?;
    let org_id = match org {
        Some(s) => Some(parse_id(&s, "--org")?),
        None => None,
    };
    let projects = ctx.api.projects_list(org_id).await?;

    if ctx.json {
        return output::print_json(&projects);
    }

    if projects.is_empty() {
        println!("No projects.");
        return Ok(());
    }

    println!(
        "{:<6}  {:<24}  {:<20}  {:<12}  {:<6}  {:<6}  {}",
        "ID", "NAME", "STATUS", "PLAN", "ORG", "DC", "CREATED"
    );
    for p in projects {
        println!(
            "{:<6}  {:<24}  {:<20}  {:<12}  {:<6}  {:<6}  {}",
            p.id,
            truncate(&p.name, 24),
            truncate(&p.status, 20),
            truncate(p.plan.as_deref().unwrap_or("-"), 12),
            opt_id(p.organization_id),
            opt_id(p.datacenter_id),
            p.created_at.unwrap_or_else(|| "-".into()),
        );
    }
    Ok(())
}

async fn show(ctx: &Context, project_id: &str) -> Result<()> {
    require_auth(ctx)?;
    let id = parse_id(project_id, "project_id")?;
    let project = ctx.api.projects_show(id).await?;

    if ctx.json {
        return output::print_json(&project);
    }
    print_project(&project);
    Ok(())
}

async fn delete(ctx: &Context, project_id: &str) -> Result<()> {
    require_auth(ctx)?;
    let id = parse_id(project_id, "project_id")?;
    ctx.api.projects_delete(id).await?;

    if ctx.json {
        output::print_json(&serde_json::json!({ "deleted": id }))?;
    } else {
        println!("Project {id} deleted.");
    }
    Ok(())
}

async fn create(
    ctx: &Context,
    name: String,
    datacenter: String,
    plan: String,
    org: String,
    repos: Vec<String>,
) -> Result<()> {
    require_auth(ctx)?;
    let datacenter_id = parse_id(&datacenter, "--datacenter")?;
    let org_id = parse_id(&org, "--org")?;

    let result = ctx
        .api
        .projects_create(&name, &plan, datacenter_id, org_id, &repos)
        .await?;

    if ctx.json {
        return output::print_json(&result);
    }

    print_project(&result.project);
    if let Some(url) = result.checkout_url.as_deref() {
        println!("Checkout:     {url}");
    }

    let pending = result.project.checkout_status.as_deref() == Some("payment_pending");
    let checkout_url = result.checkout_url.clone();
    if !pending || checkout_url.is_none() {
        if !pending {
            println!("Project {} ready.", result.project.id);
        }
        return Ok(());
    }

    let url = checkout_url.unwrap();
    println!();
    println!("Opening checkout in your browser...");
    let _ = webbrowser::open(&url);

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message("Waiting for payment to complete...");
    pb.enable_steady_tick(Duration::from_millis(120));

    let project_id = result.project.id;
    let deadline = Instant::now() + Duration::from_secs(600);
    let interval = Duration::from_secs(3);

    loop {
        if Instant::now() >= deadline {
            pb.finish_and_clear();
            println!(
                "Timed out waiting for payment. Re-run `nvg projects show {project_id}` later."
            );
            return Ok(());
        }
        tokio::time::sleep(interval).await;
        match ctx.api.projects_show(project_id).await {
            Ok(p) => {
                if p.checkout_status.as_deref() != Some("payment_pending") {
                    pb.finish_and_clear();
                    println!("Project {project_id} ready (status: {}).", p.status);
                    return Ok(());
                }
            }
            Err(ApiError::Network(_)) => continue,
            Err(e) => {
                pb.finish_and_clear();
                return Err(anyhow::Error::from(e));
            }
        }
    }
}

fn print_project(p: &InfrastructureProject) {
    println!("ID:           {}", p.id);
    println!("Name:         {}", p.name);
    println!("Status:       {}", p.status);
    println!("Plan:         {}", p.plan.as_deref().unwrap_or("-"));
    println!("Organization: {}", opt_id(p.organization_id));
    println!("Datacenter:   {}", opt_id(p.datacenter_id));
    println!(
        "Created:      {}",
        p.created_at.as_deref().unwrap_or("-")
    );
}

fn parse_id(s: &str, label: &str) -> Result<i64> {
    s.parse::<i64>()
        .with_context(|| format!("{label} must be a numeric id (got {s:?})"))
}

fn opt_id(id: Option<i64>) -> String {
    id.map(|n| n.to_string()).unwrap_or_else(|| "-".into())
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
