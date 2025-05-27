use crate::{Context, Error};

/// Stops any game you created.
#[poise::command(
    slash_command,
    user_cooldown = 3,
    name_localized("ja", "止まる"),
    description_localized("ja", "ゲームを止まる")
)]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    let stopped = ctx.data().manager.stop(ctx.author().id).await;

    if stopped {
        ctx.say("Stopping game...").await?;
    } else {
        ctx.say("There is no running game to stop").await?;
    }

    Ok(())
}
