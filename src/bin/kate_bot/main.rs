use std::sync::Arc;

use dotenvy::dotenv;
use poise::{
    Framework,
    serenity_prelude::{self as serenity, GuildId},
};

mod command;
mod dictionary;
mod game;
mod image;
mod emote;

pub struct Data {
    pub manager: Arc<game::Manager>,
}
pub type Context<'a> = poise::Context<'a, Data, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let token = std::env::var("DISCORD_TOKEN").expect("Missing `DISCORD_TOKEN` env var.");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework: Framework<Data, Error> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![command::start(), command::stop(), command::info()],
            event_handler: |_ctx, event, framework, _data| {
                Box::pin(event_handler(event.clone(), framework))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                if let Ok(guild_id) =
                    std::env::var("DISCORD_DEV_GUILD_ID").map(|v| v.parse().unwrap())
                {
                    println!("Registering commands to DEV Guild");
                    poise::builtins::register_in_guild(
                        ctx,
                        &framework.options().commands,
                        GuildId::new(guild_id),
                    )
                    .await?;
                } else {
                    println!("Registering commands globally");
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                }

                Ok(Data {
                    manager: game::Manager::new(ctx.http.clone()).into(),
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}

async fn event_handler(
    event: serenity::FullEvent,
    framework: poise::FrameworkContext<'_, Data, Error>,
) -> Result<(), Error> {
    if let serenity::FullEvent::InteractionCreate { interaction } = event {
        if let Some(interaction) = interaction.into_message_component() {
            framework.user_data.manager.send(interaction).await;
        };
    };

    Ok(())
}
