use std::time::Duration;

use crate::{Context, Error};
use jplearnbot::dictionary::NLevel;
use poise::serenity_prelude::{
    ComponentInteractionCollector, ComponentInteractionDataKind, CreateActionRow, CreateButton,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption, futures::StreamExt,
};
use strum::IntoEnumIterator;
use tokio::time::sleep;

#[derive(Debug, poise::ChoiceParameter)]
enum ModeChoice {
    #[name = "English ▶ ひらがな"]
    EngToHir,
    #[name = "ひらがな ▶ English"]
    HirToEng,
    #[name = "ひらがな ▶ 漢字"]
    HirToKan,
    #[name = "漢字 ▶ ひらがな"]
    KanToHir,
}

/// Starts a new game
#[poise::command(slash_command, user_cooldown = 3)]
pub async fn start(
    ctx: Context<'_>,
    #[description = "Game mode"] mode: ModeChoice,
) -> Result<(), Error> {
    let mut menu = FiltersMenu::new(ctx.id(), mode);

    ctx.send(
        poise::CreateReply::default()
            .components(menu.create_components())
            .ephemeral(true),
    )
    .await?;

    menu.handle_interactions(&ctx).await?;

    Ok(())
}

/// Manages the components of the create game form.
struct FiltersMenu {
    /// Identifier for the NLevel filter menu.
    nlvls_id: String,
    /// Currently selected NLevels. Initially all of them.
    nlvls: Vec<NLevel>,

    /// Identifier for the parts of speech filter menu.
    pos_id: String,
    /// Currently selected parts of speech. Initially all of them.
    pos: Vec<String>,

    /// Identifier for the submit button.
    submit_id: String,

    /// Mode of game to create.
    mode: ModeChoice,
}

impl FiltersMenu {
    fn new(invocation_id: u64, mode: ModeChoice) -> Self {
        let id = invocation_id.to_string();
        FiltersMenu {
            nlvls_id: format!("{}-nlvls", id),
            nlvls: NLevel::iter().collect(),

            pos_id: format!("{}-pos", id),
            pos: vec![
                "Nouns".to_string(),
                "Verbs".to_string(),
                "Prenominals".to_string(),
            ],

            submit_id: format!("{}-submit", id),

            mode,
        }
    }

    /// Create all of the components of this menu.
    fn create_components(&self) -> Vec<CreateActionRow> {
        vec![self.nlvls_menu(), self.pos_menu(), self.submit_button()]
    }

    /// Creates a new menu for selecting NLevels. Used by [`Self::create_components`].
    fn nlvls_menu(&self) -> CreateActionRow {
        let nlvls = self
            .nlvls
            .iter()
            .map(|lvl| {
                CreateSelectMenuOption::new(lvl.to_string(), lvl.to_string())
                    .default_selection(true)
            })
            .collect::<Vec<_>>();
        let nlvls_len = nlvls.len();

        let menu = CreateSelectMenu::new(
            &self.nlvls_id,
            CreateSelectMenuKind::String { options: nlvls },
        )
        .placeholder("Select NLevel Pool(s)")
        .min_values(1)
        .max_values(nlvls_len.try_into().expect("Too many options were added"));

        CreateActionRow::SelectMenu(menu)
    }

    /// Creates a new menu for selecting parts of speech. Used by [`Self::create_components`].
    fn pos_menu(&self) -> CreateActionRow {
        let pos = self
            .pos
            .iter()
            .map(|p| CreateSelectMenuOption::new(p, p).default_selection(true))
            .collect::<Vec<_>>();
        let pos_len = pos.len();

        let menu =
            CreateSelectMenu::new(&self.pos_id, CreateSelectMenuKind::String { options: pos })
                .placeholder("Select parts-of-speech filters")
                .min_values(1)
                .max_values(pos_len.try_into().expect("Too many options were added"));

        CreateActionRow::SelectMenu(menu)
    }

    /// Creates a new submit button. Used by [`Self::create_components`].
    fn submit_button(&self) -> CreateActionRow {
        let button = CreateButton::new(&self.submit_id).label("Create Game");
        CreateActionRow::Buttons(vec![button])
    }

    /// Listens for form interactions. Starts a game on submission. Does nothing
    /// on subsequent submissions.
    async fn handle_interactions(&mut self, ctx: &Context<'_>) -> Result<(), Error> {
        let mut submitted = false;

        let mut collector = ComponentInteractionCollector::new(ctx)
            .author_id(ctx.author().id)
            .channel_id(ctx.channel_id())
            .timeout(Duration::from_secs(20))
            .filter({
                // Only listen for this form's components.
                let ids = [
                    self.nlvls_id.clone(),
                    self.pos_id.clone(),
                    self.submit_id.clone(),
                ];
                move |mci| ids.contains(&mci.data.custom_id)
            })
            .stream();

        while let Some(ci) = collector.next().await {
            let id = &ci.data.custom_id;

            match &ci.data.kind {
                // Update filters
                ComponentInteractionDataKind::StringSelect { values } => {
                    if id != &self.nlvls_id || id != &self.pos_id {
                        continue;
                    }

                    if id == &self.nlvls_id {
                        self.nlvls = values.iter().map(|v| v.parse().unwrap()).collect();
                    } else if id == &self.pos_id {
                        self.pos = values.iter().map(|v| v.parse().unwrap()).collect();
                    }
                    ci.create_response(ctx, CreateInteractionResponse::Acknowledge)
                        .await?;
                }
                // Handle submit
                ComponentInteractionDataKind::Button => {
                    if id != &self.submit_id {
                        continue;
                    }

                    // Ignore subsequent submissions
                    if submitted {
                        ci.create_response(
                            ctx,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .content("Game has already been created")
                                    .ephemeral(true),
                            ),
                        )
                        .await?;
                        continue;
                    }
                    submitted = true;

                    /// TODO: Find out what this reply looks like to an ephmereal create game form
                    ci.create_response(
                        ctx,
                        CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
                    )
                    .await?;

                    sleep(Duration::from_secs(3)).await;

                    ci.delete_response(ctx).await?;

                    ci.channel_id
                        .send_message(ctx, CreateMessage::new().content("unsolicited"))
                        .await?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
