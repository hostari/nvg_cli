use crate::api::error::ApiError;
use crate::cli::OrgsCommand;
use crate::commands::Context;
use crate::output;
use anyhow::Result;

pub async fn run(cmd: OrgsCommand, ctx: &Context) -> Result<()> {
    match cmd {
        OrgsCommand::List => list(ctx).await,
    }
}

async fn list(ctx: &Context) -> Result<()> {
    require_auth(ctx)?;
    let orgs = ctx.api.organizations_list().await?;

    if ctx.json {
        return output::print_json(&orgs);
    }

    if orgs.is_empty() {
        println!("No organizations.");
        return Ok(());
    }

    println!("{:<8}  {}", "ID", "NAME");
    for o in orgs {
        println!("{:<8}  {}", o.id, o.name);
    }
    Ok(())
}

fn require_auth(ctx: &Context) -> Result<()> {
    if ctx.profile.token.is_none() {
        return Err(anyhow::Error::from(ApiError::Unauthorized));
    }
    Ok(())
}
