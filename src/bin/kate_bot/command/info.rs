use crate::{Context, Error};

/// Information about the bot.
#[poise::command(
    slash_command,
    user_cooldown = 3,
    name_localized("ja", "情報"),
    description_localized("ja", "ボットの情報")
)]
pub async fn info(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Art - <https://x.com/matcha__ore_p/>\nQuestions/Feedback - (discord) sweetenedlegs")
        .await?;

    Ok(())
}
