use std::{
    fmt::Display,
    sync::{Arc, LazyLock},
    time::Duration,
};

use const_format::formatcp;
use dashmap::DashMap;
use jplearnbot::dictionary::{DictEntry, NLevel, Pos};
use poise::serenity_prelude::{
    ComponentInteraction, CreateActionRow, CreateButton, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditMessage, UserId, http::Http,
};
use rand::{rng, seq::IndexedRandom};
use regex::Regex;
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    time::timeout,
};
use uuid::Uuid;

use crate::{Context, dictionary::Dictionary};

#[derive(Debug, poise::ChoiceParameter, Clone, Copy)]
pub enum ModeChoice {
    #[name = "English ▶ ひらがな"]
    EngToHir,
    #[name = "ひらがな ▶ English"]
    HirToEng,
    #[name = "ひらがな ▶ 漢字"]
    HirToKan,
    #[name = "漢字 ▶ ひらがな"]
    KanToHir,
}

pub enum GameMessage {
    /// A component interaction.
    Interaction(ComponentInteraction),
    /// Indicates game should close.
    Close,
}

/// Manages all game sessions.
pub struct Manager {
    /// Handle to serenity client.
    http: Arc<Http>,
    /// Dictionary for getting randomized samples and entries.
    dictionary: Arc<Dictionary>,
    /// Stores transmitters to game sessions. Mapped to each UserId
    /// because users may have exactly one running game session.
    sessions: Arc<DashMap<UserId, Sender<GameMessage>>>,
}

impl Manager {
    pub fn new(http: Arc<Http>) -> Self {
        Manager {
            http,
            dictionary: Dictionary::new().into(),
            sessions: DashMap::new().into(),
        }
    }

    pub fn start_game(
        &self,
        ctx: &Context<'_>,
        mode: ModeChoice,
        levels: Vec<NLevel>,
        pos: Vec<Pos>,
    ) -> Result<(), SessionAlreadyCreated> {
        let user_id = ctx.author().id;
        if self.sessions.contains_key(&user_id) {
            return Err(SessionAlreadyCreated);
        }

        let channel_id = ctx.channel_id();

        let http = Arc::clone(&self.http);
        let sessions = Arc::clone(&self.sessions);
        let dictionary = Arc::clone(&self.dictionary);

        let (tx, mut rx) = mpsc::channel(10);
        self.sessions.insert(user_id, tx);

        tokio::spawn(async move {
            let mut exit_reason = InteractionExitReason::PoolExhausted;

            for (round, entry) in dictionary.sample(&levels, &pos).await.iter().enumerate() {
                let Some(question) = Question::new(entry, mode, &dictionary) else {
                    continue;
                };

                let menu_id = format!("{user_id},{}", Uuid::new_v4());
                let mut menu = Menu::new(&http, menu_id, question);

                if channel_id
                    .send_message(
                        &http,
                        CreateMessage::new()
                            .content(format!("Round {round}\nId: {}", entry.id))
                            .components(menu.create_components()),
                    )
                    .await
                    .is_err()
                {
                    exit_reason = InteractionExitReason::NetworkError;
                    break;
                }

                if let Err(reason) = menu.handle_interactions(&mut rx).await {
                    exit_reason = reason;
                    break;
                }
            }

            let message = match exit_reason {
                InteractionExitReason::PoolExhausted => {
                    Some("There are no more words left in the pool")
                }
                InteractionExitReason::Timeout => Some("Stopping game due to inactivity..."),
                InteractionExitReason::NetworkError => {
                    Some("Stopping game due to network error...")
                }
                InteractionExitReason::CloseRequest => None,
            };

            if let Some(message) = message {
                channel_id
                    .send_message(&http, CreateMessage::new().content(message))
                    .await
                    .ok();
            }

            sessions.remove(&user_id);
        });

        Ok(())
    }

    /// Stops `user_id`'s game if it exists.
    ///
    /// Returns true if there was a running game stopped.
    ///
    /// Returns false if there was no game associated with the user.
    pub async fn stop(&self, user_id: UserId) -> bool {
        if let Some(tx) = self.sessions.get(&user_id) {
            tx.send(GameMessage::Close).await.ok();
            return true;
        }

        false
    }

    /// Sends `interaction` to the game session compatible with the interaction's custom_id.
    /// Does nothing if no matching game sesssion.
    pub async fn send(&self, interaction: ComponentInteraction) {
        if let Some(tx) =
            parse_user_id(&interaction.data.custom_id).and_then(|id| self.sessions.get(&id))
        {
            tx.send(GameMessage::Interaction(interaction)).await.ok();
        }
    }
}

enum InteractionExitReason {
    /// There are no more words left in the pool.
    PoolExhausted,
    /// Sender took too long to send a message.
    Timeout,
    /// Error sending data to Discord.
    NetworkError,
    /// Game should close.
    CloseRequest,
}

/// Extracts UserId from interaction's custom_id.
fn parse_user_id(interaction_id: &str) -> Option<UserId> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d+").unwrap());

    let user_id = RE.find(interaction_id)?.as_str().parse().ok()?;

    Some(UserId::new(user_id))
}

#[derive(Debug)]
pub struct SessionAlreadyCreated;

impl Display for SessionAlreadyCreated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "User has already created a session")
    }
}

impl std::error::Error for SessionAlreadyCreated {}

struct Question {
    options: [String; 4],
    answer: usize,
}

impl Question {
    fn new(entry: &DictEntry, mode: ModeChoice, dictionary: &Dictionary) -> Option<Self> {
        // Some(Question {
        //     answer: 0,
        //     options: [
        //         "One".to_string(),
        //         "Two".to_string(),
        //         "Three".to_string(),
        //         "Four".to_string(),
        //     ],
        // })
        match mode {
            ModeChoice::EngToHir => Self::new_eng_to_hir(entry, dictionary),
            ModeChoice::HirToEng => Self::new_hir_to_eng(entry, dictionary),
            ModeChoice::HirToKan => Self::new_hir_to_kan(entry, dictionary),
            ModeChoice::KanToHir => Self::new_kan_to_hir(entry, dictionary),
        }
    }

    fn new_eng_to_hir(entry: &DictEntry, dictionary: &Dictionary) -> Option<Self> {
        todo!()
    }

    fn new_hir_to_eng(entry: &DictEntry, dictionary: &Dictionary) -> Option<Self> {
        todo!()
    }

    fn new_hir_to_kan(entry: &DictEntry, dictionary: &Dictionary) -> Option<Self> {
        todo!()
    }

    fn new_kan_to_hir(entry: &DictEntry, dictionary: &Dictionary) -> Option<Self> {
        todo!()
    }

    fn answer(&self) -> &str {
        &self.options[self.answer]
    }
}

struct Menu<'a> {
    id: String,
    questions: Vec<QuestionComponent>,
    answer: usize,
    http: &'a Http,
}

struct QuestionComponent {
    id: String,
    text: String,
    disabled: bool,
}

impl<'a> Menu<'a> {
    fn new(http: &'a Http, id: String, question: Question) -> Self {
        let questions = question
            .options
            .into_iter()
            .enumerate()
            .map(|(i, text)| QuestionComponent {
                id: format!("{id},{i}"),
                text,
                disabled: false,
            })
            .collect();

        Menu {
            id,
            questions,
            answer: question.answer,
            http,
        }
    }

    /// Equivalent to [`Self::questions`][].[`id`]
    fn answer_id(&self) -> &str {
        &self.questions[self.answer].id
    }

    /// Create all of the components of this menu.
    fn create_components(&self) -> Vec<CreateActionRow> {
        let buttons = self
            .questions
            .iter()
            .map(|q| CreateButton::new(&q.id).label(&q.text).disabled(q.disabled))
            .collect();

        vec![CreateActionRow::Buttons(buttons)]
    }

    /// Listens for button interactions until the answer is chosen.
    async fn handle_interactions(
        &mut self,
        rx: &mut Receiver<GameMessage>,
    ) -> Result<(), InteractionExitReason> {
        loop {
            let mut ci = component_interaction(rx).await?;

            let Some((menu_id, choice)) = parse_custom_id(&ci.data.custom_id) else {
                continue;
            };
            // Skip if menu_id of previous round.
            if menu_id != self.id {
                continue;
            }

            let correct = self.questions[choice].id == self.answer_id();

            // If correct, disable all buttons since this round is finished.
            // Otherwise, just the wrongly selected button.
            if correct {
                self.questions.iter_mut().for_each(|q| q.disabled = true);
            } else {
                self.questions[choice].disabled = true;
            }

            ci.message
                .edit(
                    self.http,
                    EditMessage::new().components(self.create_components()),
                )
                .await
                .map_err(|_| InteractionExitReason::NetworkError)?;

            ci.create_response(
                self.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content(if correct {
                        format!("<:correct:1375301870200819932> correct <@{}>", ci.user.id)
                    } else {
                        insult_message(ci.user.id)
                    }),
                ),
            )
            .await
            .map_err(|_| InteractionExitReason::NetworkError)?;

            if correct {
                break;
            }
        }

        Ok(())
    }
}

/// Unwraps component interactions from `rx`.
///
/// Returns [`InteractionExitReason::Timeout`] if sender takes
/// too long.
///
/// Returns [`InteractionExitReason::CloseRequest`] if sender sends
/// [`GameMessage::Close`].
async fn component_interaction(
    rx: &mut Receiver<GameMessage>,
) -> Result<ComponentInteraction, InteractionExitReason> {
    let Ok(Some(msg)) = timeout(Duration::from_secs(120), rx.recv()).await else {
        return Err(InteractionExitReason::Timeout);
    };

    let GameMessage::Interaction(ci) = msg else {
        return Err(InteractionExitReason::CloseRequest);
    };

    Ok(ci)
}

/// Parses a component's custom_id for its menu_id and the user's button choice.
fn parse_custom_id(custom_id: &str) -> Option<(&str, usize)> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.*),([0-3])$").unwrap());

    RE.captures(custom_id)
        .and_then(|m| Some((m.get(1)?, m.get(2)?))) // Get capture groups
        .and_then(|(menu_id, choice)| {
            Some((menu_id.as_str(), choice.as_str().parse().ok()?)) // Convert format
        })
}

/// Creates a randomized insult message that mentions `user_id`.
fn insult_message(user_id: UserId) -> String {
    const FUBU_LAUGH_EMOTE: &str = "<:fubu_laugh:1375302817778106490>";
    const SCRAJJ_EMOTE: &str = "<a:scrajj:1375305497267146874>";
    const ANW_EMOTE: &str = "<a:aintnoway:1375305628444004473>";
    const WAT_EMOTE: &str = "<:wat:1373080615313739858>";

    const INSULTS: [&str; 20] = [
        formatcp!("{WAT_EMOTE} noob"),
        formatcp!("{WAT_EMOTE} nuh-uh"),
        formatcp!("{WAT_EMOTE} what is he cooking"),
        formatcp!("{WAT_EMOTE} refund nitro"),
        formatcp!("{WAT_EMOTE} trolling are we?"),
        formatcp!("{WAT_EMOTE} nt bro"),
        formatcp!("{WAT_EMOTE} smooth brain"),
        formatcp!("{WAT_EMOTE} stop"),
        formatcp!("{WAT_EMOTE} ?"),
        formatcp!("{WAT_EMOTE} so bad"),
        formatcp!("{WAT_EMOTE} meow"),
        formatcp!("{WAT_EMOTE} imagine"),
        formatcp!("{WAT_EMOTE} no"),
        formatcp!("{WAT_EMOTE} wrong"),
        formatcp!("{WAT_EMOTE} ぴえん"),
        formatcp!("{WAT_EMOTE} あほ"),
        WAT_EMOTE,
        FUBU_LAUGH_EMOTE,
        SCRAJJ_EMOTE,
        ANW_EMOTE,
    ];

    let insult = INSULTS.choose(&mut rng()).unwrap();

    format!("{insult} <@{user_id}>")
}
