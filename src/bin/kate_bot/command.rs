use poise::serenity_prelude as serenity;

mod start;
pub use start::*;

use crate::Context;

pub fn cinteraction_collector(
    component_id: &str,
    ctx: &Context<'_>,
    timeout: u64,
) -> serenity::ComponentInteractionCollector {
    let id = component_id.to_string();
    serenity::ComponentInteractionCollector::new(ctx)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(timeout))
        .filter(move |mci| mci.data.custom_id == id)
}
