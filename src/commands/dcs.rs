use crate::api::error::ApiError;
use crate::cli::DcsCommand;
use crate::commands::Context;
use crate::output;
use anyhow::Result;

pub async fn run(cmd: DcsCommand, ctx: &Context) -> Result<()> {
    match cmd {
        DcsCommand::List => list(ctx).await,
    }
}

async fn list(ctx: &Context) -> Result<()> {
    require_auth(ctx)?;
    let dcs = ctx.api.datacenters_list().await?;

    if ctx.json {
        return output::print_json(&dcs);
    }

    if dcs.is_empty() {
        println!("No datacenters.");
        return Ok(());
    }

    println!(
        "{:<8}  {:<3}  {:<20}  {:<20}  {}",
        "ID", "CC", "REGION", "CITY", "ACTIVE"
    );
    for d in dcs {
        println!(
            "{:<8}  {:<3}  {:<20}  {:<20}  {}",
            d.id,
            d.country,
            truncate(&d.region, 20),
            truncate(&d.city, 20),
            if d.active { "yes" } else { "no" },
        );
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
