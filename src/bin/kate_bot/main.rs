use std::sync::Arc;

use dotenvy::dotenv;
use poise::{
    Framework,
    serenity_prelude::{self as serenity, GuildId},
};

mod command;
mod dictionary;
mod game;

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
            commands: vec![command::start(), command::stop()],
            event_handler: |_ctx, event, framework, _data| {
                Box::pin(event_handler(event.clone(), framework))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    GuildId::new(1148122592293699584),
                )
                .await?;
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
