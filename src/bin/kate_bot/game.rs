use std::{
    fmt::Display,
    sync::{Arc, LazyLock},
    time::Duration,
};

use const_format::formatcp;
use dashmap::DashMap;
use jplearnbot::dictionary::{DictEntry, Kanji, NLevel, Pos, Reading, Sense};
use poise::serenity_prelude::{
    ComponentInteraction, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, EditMessage,
    UserId, http::Http,
};
use rand::{
    distr::{Bernoulli, Distribution},
    rng,
    seq::{IndexedRandom, IteratorRandom, SliceRandom},
};
use regex::Regex;
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    time::timeout,
};
use uuid::Uuid;

use crate::{Context, dictionary::Dictionary};

/// Game modes
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

    /// Starts a new game session with the selected `mode`, `levels`, and `pos`.
    /// A separate task is created for game interaction handling. A [`Sender`]
    /// to the session is stored in [`Self::sessions`] for the duration of the game.
    /// The sessions exists while there are words in the pool and user interaction
    /// doesn't timeout from inactivity. A session can be stopped prematurely by sending
    /// a [`GameMessage::Close`] through the sender.
    ///
    /// # Errors
    /// Fails if user already has a running game.
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

        let mut pos = pos;

        let (tx, mut rx) = mpsc::channel(10);
        self.sessions.insert(user_id, tx);

        tokio::spawn(async move {
            // Natural expected exit reason, reason may change from interactions or lack thereof.
            let mut exit_reason = InteractionExitReason::PoolExhausted;

            for (round, entry) in dictionary.sample(&levels, &pos).await.iter().enumerate() {
                pos.shuffle(&mut rng());
                let Some(question) = pos
                    .iter()
                    .find_map(|&p| Question::new(entry, mode, p, &dictionary))
                else {
                    continue;
                };

                let menu_id = format!("{user_id},{}", Uuid::new_v4());
                let mut menu = Menu::new(&http, menu_id, question, entry);

                if channel_id
                    .send_message(
                        &http,
                        CreateMessage::new()
                            .content(format!(
                                "Round {round}\nId: {}\nPrompt: {}",
                                entry.id, menu.prompt
                            ))
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

/// Reasons listening for component interactions should stop.
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

/// Game question
struct Question {
    /// The word to translate.
    prompt: String,
    /// Possible translations of [`Self::prompt`].
    options: [String; 5],
    /// The index of the correct translation of [`Self::prompt`].
    answer: usize,
}

impl Question {
    fn new(entry: &DictEntry, mode: ModeChoice, pos: Pos, dictionary: &Dictionary) -> Option<Self> {
        match mode {
            ModeChoice::EngToHir => Self::new_eng_to_hir(entry, pos),
            ModeChoice::HirToEng => Self::new_hir_to_eng(entry, pos, dictionary),
            ModeChoice::HirToKan => Self::new_hir_to_kan(entry, pos, dictionary),
            ModeChoice::KanToHir => Self::new_kan_to_hir(entry, pos),
        }
    }

    fn new_eng_to_hir(entry: &DictEntry, pos: Pos) -> Option<Self> {
        let (reading, sense) = reading_sense_pair(entry, pos)?;

        let (answer, options) = create_reading_options(reading.text.clone());

        Some(Question {
            prompt: sense.gloss[0].content.clone(),
            options,
            answer,
        })
    }

    fn new_hir_to_eng(entry: &DictEntry, pos: Pos, dictionary: &Dictionary) -> Option<Self> {
        let (reading, sense) = reading_sense_pair(entry, pos)?;

        let mut options = std::array::from_fn(|_| "".to_string());
        options[0] = sense.gloss[0].content.clone();

        dictionary
            .entries
            .iter()
            .filter_map(|e| {
                if e.id == entry.id {
                    return None;
                }

                for sense in &e.senses {
                    if !sense.gloss.is_empty() && sense.pos.contains(&pos) {
                        return Some(sense.gloss[0].content.clone());
                    }
                }

                None
            })
            .choose_multiple_fill(&mut rng(), &mut options[1..]);

        options.shuffle(&mut rng());

        let answer = options
            .iter()
            .position(|o| sense.gloss[0].content == *o)
            .unwrap();

        Some(Question {
            prompt: reading.text.clone(),
            options,
            answer,
        })
    }

    fn new_hir_to_kan(entry: &DictEntry, pos: Pos, dictionary: &Dictionary) -> Option<Self> {
        let (kanji, reading) = kanji_reading_pair(entry, pos)?;

        let mut options = std::array::from_fn(|_| "".to_string());
        options[0] = kanji.text.clone();
        dictionary
            .entries
            .iter()
            .filter_map(|e| {
                if e.id == entry.id {
                    return None;
                }

                if !e.senses.iter().any(|s| s.pos.contains(&pos)) {
                    return None;
                }

                Some(e.kanjis.first()?.text.clone())
            })
            .choose_multiple_fill(&mut rng(), &mut options[1..]);

        options.shuffle(&mut rng());

        let answer = options.iter().position(|o| kanji.text == *o).unwrap();

        Some(Question {
            prompt: reading.text.clone(),
            options,
            answer,
        })
    }

    fn new_kan_to_hir(entry: &DictEntry, pos: Pos) -> Option<Self> {
        let (kanji, reading) = kanji_reading_pair(entry, pos)?;

        let (answer, options) = create_reading_options(reading.text.clone());

        Some(Question {
            prompt: kanji.text.clone(),
            options,
            answer,
        })
    }

    /// Convenience getter for the correct translation of the [`Self::prompt`].
    fn answer(&self) -> &str {
        &self.options[self.answer]
    }
}

/// Conventiently extracts a [`Reading`] and correlated [`Sense`] from a [`DictEntry`] where
/// the sense has the `pos` tag and is guaranteed to have at least one gloss.
///
/// Returns [`None`] if no possible extraction.
fn reading_sense_pair(entry: &DictEntry, pos: Pos) -> Option<(&Reading, &Sense)> {
    let sense = entry
        .senses
        .iter()
        .find(|s| s.pos.contains(&pos) && !s.gloss.is_empty())?;

    let reading = entry
        .readings
        .iter()
        .find(|r| sense.relevant_reading.is_empty() || sense.relevant_reading.contains(&r.text))?;

    Some((reading, sense))
}

/// Creates an array containing `reading` and scrambled versions of `reading`.
/// If it takes too many tries to create a unique scrambled version of `reading`.
/// "OPTION" is added to the array instead.
///
/// Returns an index to the original `reading` and the array.
fn create_reading_options(reading: String) -> (usize, [String; 5]) {
    let mut res = std::array::from_fn(|_| "".to_string());
    res[0] = reading.clone();

    'outer: for i in 1..res.len() {
        const MAX_SCRAMBLE_TRIES: i32 = 1000;

        for _ in 0..MAX_SCRAMBLE_TRIES {
            let scrambled = scrambled(&reading);
            if !res.contains(&scrambled) {
                res[i] = scrambled;
                continue 'outer;
            }
        }

        res[i] = "OPTION".to_string();
    }
    res.shuffle(&mut rng());

    let original_idx = res.iter().position(|r| reading == *r).unwrap();

    (original_idx, res)
}

/// Scrambles the hiragana or katakana of `reading`.
fn scrambled(reading: &str) -> String {
    let mut rng = rng();
    let bern = Bernoulli::new(0.5).unwrap();
    let always_swap = reading.chars().count() < 4 || reading::swappable_ratio(reading) < 0.6;

    reading
        .chars()
        .map(|c| {
            if let Some(pool) = reading::swap_pool(c) {
                if always_swap || bern.sample(&mut rng) {
                    return *pool.choose(&mut rng).unwrap_or(&c);
                }
            }
            c
        })
        .collect()
}

fn kanji_reading_pair(entry: &DictEntry, pos: Pos) -> Option<(&Kanji, &Reading)> {
    let sense = entry.senses.iter().find(|s| s.pos.contains(&pos))?;

    let kanji = entry.kanjis.first()?;

    let reading = entry.readings.iter().find(|r| {
        (r.relevant_to.is_empty() || r.relevant_to.contains(&kanji.text))
            && (sense.relevant_reading.is_empty() || sense.relevant_reading.contains(&r.text))
    })?;

    Some((kanji, reading))
}

mod reading {
    use std::ops::Deref;

    struct Chart([[char; 5]; 8]);

    impl Deref for Chart {
        type Target = [[char; 5]; 8];
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    const HIRA_CHART: Chart = Chart([
        ['あ', 'い', 'う', 'え', 'お'],
        ['か', 'き', 'く', 'け', 'こ'],
        ['さ', 'し', 'す', 'せ', 'そ'],
        ['た', 'ち', 'つ', 'て', 'と'],
        ['な', 'に', 'ぬ', 'ね', 'の'],
        ['は', 'ひ', 'ふ', 'へ', 'ほ'],
        ['ま', 'み', 'む', 'め', 'も'],
        ['ら', 'り', 'る', 'れ', 'ろ'],
    ]);
    const HIRA_WA_COL: [char; 10] = ['わ', 'あ', 'か', 'さ', 'た', 'な', 'は', 'ま', 'や', 'ら'];
    const HIRA_Y_ROW: [char; 3] = ['や', 'ゆ', 'よ'];
    
    const KATA_CHART: Chart = Chart([
        ['ア', 'イ', 'ウ', 'エ', 'オ'],
        ['カ', 'キ', 'ク', 'ケ', 'コ'],
        ['サ', 'シ', 'ス', 'セ', 'ソ'],
        ['タ', 'チ', 'ツ', 'テ', 'ト'],
        ['ナ', 'ニ', 'ヌ', 'ネ', 'ノ'],
        ['ハ', 'ヒ', 'フ', 'ヘ', 'ホ'],
        ['マ', 'ミ', 'ム', 'メ', 'モ'],
        ['ラ', 'リ', 'ル', 'レ', 'ロ'],
    ]);
    const KATA_WA_COL: [char; 10] = ['ワ', 'ア', 'カ', 'サ', 'タ', 'ナ', 'ハ', 'マ', 'ヤ', 'ラ'];
    const KATA_Y_ROW: [char; 3] = ['ヤ', 'ユ', 'ヨ'];

    /// Finds possible Hiragana or Katakana that can replace `char` in a scramble.
    pub fn swap_pool(char: char) -> Option<Vec<char>> {
        if let Some(pool) = chart_swap_pool(char, &HIRA_CHART) {
            return Some(pool);
        }
        if char == HIRA_WA_COL[0] {
            return Some(HIRA_WA_COL[1..].to_vec());
        }
        if HIRA_Y_ROW.contains(&char) {
            return Some(HIRA_Y_ROW.iter().cloned().filter(|&h| h != char).collect());
        }

        if let Some(pool) = chart_swap_pool(char, &KATA_CHART) {
            return Some(pool);
        }
        if char == KATA_WA_COL[0] {
            return Some(KATA_WA_COL[1..].to_vec());
        }
        if KATA_Y_ROW.contains(&char) {
            return Some(KATA_Y_ROW.iter().cloned().filter(|&h| h != char).collect());
        }

        None
    }

    /// Finds possible Hiragana or Katakana that can replace `char` in a scramble
    /// in `chart`.
    fn chart_swap_pool(char: char, chart: &Chart) -> Option<Vec<char>> {
        let (row_idx, col_idx) = find_chart_coords(char, chart)?;

        let mut neighbors: Vec<char> = chart[row_idx]
            .iter()
            .cloned()
            .filter(|c| *c != char)
            .collect();

        for row in chart.iter() {
            if row[col_idx] != char {
                neighbors.push(row[col_idx]);
            }
        }

        Some(neighbors)
    }

    /// Finds the position of `char` in `chart` if it's in the chart.
    fn find_chart_coords(char: char, chart: &Chart) -> Option<(usize, usize)> {
        for (row_idx, row) in chart.iter().enumerate() {
            for (col_idx, col) in row.iter().enumerate() {
                if char == *col {
                    return Some((row_idx, col_idx));
                }
            }
        }
        None
    }

    /// Determines how many characters in `s` can be scrambled.
    pub fn swappable_ratio(s: &str) -> f64 {
        if s.is_empty() {
            return 0.0;
        }

        let (mut swappable, mut total) = (0, 0);
        for char in s.chars() {
            if swap_pool(char).is_some() {
                swappable += 1;
            }
            total += 1;
        }

        (swappable as f64) / (total as f64)
    }
}

/// Manages the components of a game question.
struct Menu<'a> {
    id: String,
    prompt: String,
    questions: Vec<QuestionComponent>,
    answer: usize,
    entry: &'a DictEntry,
    http: &'a Http,
}

/// Contains data on a game button.
struct QuestionComponent {
    /// The component's unique identifier.
    id: String,
    /// Possible translation text.
    text: String,
    /// Whether this button should be disabled.
    disabled: bool,
}

impl<'a> Menu<'a> {
    fn new(http: &'a Http, id: String, question: Question, entry: &'a DictEntry) -> Self {
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
            prompt: question.prompt,
            questions,
            answer: question.answer,
            entry,
            http,
        }
    }

    fn levels(&self) -> Vec<NLevel> {
        self.entry
            .readings
            .iter()
            .flat_map(|r| r.levels.clone())
            .collect()
    }

    /// Equivalent to [`Self::questions`]\[\].[`id`]
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

            let message = if correct {
                CreateInteractionResponseMessage::new()
                    .embed(
                        CreateEmbed::new()
                            .title("Answer · 正解")
                            .thumbnail(r"https://cdn.discordapp.com/attachments/1373332666526732359/1373337945397792788/Untitled.png?ex=6835e9a1&is=68349821&hm=0e0e29613ea5ba328b997bad120739650141649dec54f4a5dc43c35b17ff7dff&")
                            .field(format!("{} {:?}",&self.questions[self.answer].text, self.levels()), format!("[**Definition**](https://jisho.org/search/{})\n{} <:wow:1376760017486741544>", self.questions[self.answer].text, ci.user.name), false)
                    )
            } else {
                CreateInteractionResponseMessage::new().content(insult_message(ci.user.id, &self.questions[choice].text))
            };

            ci.create_response(self.http, CreateInteractionResponse::Message(message))
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
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.*),([0-4])$").unwrap());

    RE.captures(custom_id)
        .and_then(|m| Some((m.get(1)?, m.get(2)?))) // Get capture groups
        .and_then(|(menu_id, choice)| {
            Some((menu_id.as_str(), choice.as_str().parse().ok()?)) // Convert format
        })
}

/// Creates a randomized insult message that mentions `user_id`.
fn insult_message(user_id: UserId, choice: &str) -> String {
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

    format!("{insult} <@{user_id}> ({choice})")
}
