use dotenvy::dotenv;
use poise::{
    Framework,
    serenity_prelude::{self as serenity, GuildId},
};

mod command;

pub struct Data {}
pub type Context<'a> = poise::Context<'a, Data, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let token = std::env::var("DISCORD_TOKEN").expect("Missing `DISCORD_TOKEN` env var.");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework: Framework<Data, Error> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![command::start()],
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
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}
