use serde::{Deserialize, Deserializer, Serialize, de};

#[derive(Debug, Deserialize, Serialize)]
pub struct DictEntry {
    #[serde(alias = "ent_seq")]
    pub id: u32,

    #[serde(rename = "k_ele", default, skip_serializing_if = "Vec::is_empty")]
    pub kanjis: Vec<Kanji>,

    #[serde(rename = "r_ele")]
    pub readings: Vec<Reading>,

    pub sense: Vec<Sense>,
}

impl DictEntry {
    pub fn is_annotated(&self) -> bool {
        self.kanjis.iter().any(|k| k.level != NLevel::Unknown)
            || self.readings.iter().any(|r| r.level != NLevel::Unknown)
    }

    pub fn set_level(&mut self, hiragana: &str, level: NLevel) {
        let Some(reading) = self.readings.iter_mut().find(|h| h.hiragana == hiragana) else {
            return;
        };

        reading.level = level;

        // Set all Kanjis to the same JLPT level unless
        // this hiragana has a specific relevant_to list
        if reading.relevant_to.is_empty() {
            for kanji in self.kanjis.iter_mut() {
                kanji.level = level;
            }
        } else {
            for kanji in self
                .kanjis
                .iter_mut()
                .filter(|k| reading.relevant_to.contains(&k.kanji))
            {
                kanji.level = level;
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Kanji {
    #[serde(rename = "keb")]
    pub kanji: String,

    #[serde(default)]
    pub level: NLevel,

    #[serde(rename = "ke_inf", default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<KTag>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Reading {
    #[serde(rename = "reb")]
    pub hiragana: String,

    #[serde(rename = "re_restr", default, skip_serializing_if = "Vec::is_empty")]
    pub relevant_to: Vec<String>,

    #[serde(default)]
    pub level: NLevel,

    #[serde(rename = "re_inf", default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<RTag>,
}

#[derive(Debug, Deserialize, Serialize, Default, PartialEq, Clone, Copy)]
pub enum NLevel {
    #[default]
    Unknown,
    One,
    Two,
    Three,
    Four,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum KTag {
    #[serde(rename = "&ateji;")]
    Ateji,
    #[serde(rename = "&ik;")]
    IrKana,
    #[serde(rename = "&iK;")]
    IrKanji,
    #[serde(rename = "&io;")]
    IrOkurigana,
    #[serde(rename = "&oK;")]
    Outdated,
    #[serde(rename = "&rK;")]
    Rare,
    #[serde(rename = "&sK;")]
    SearchOnly,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum RTag {
    #[serde(rename = "&gikun;")]
    Gikun,
    #[serde(rename = "&ik;")]
    IrKana,
    #[serde(rename = "&ok;")]
    Outdated,
    #[serde(rename = "&sk;")]
    SearchOnly,
    #[serde(rename = "&rk;")]
    Archaic,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Sense {
    #[serde(rename = "stagk", default, skip_serializing_if = "Vec::is_empty")]
    pub relevant_kanji: Vec<String>,

    #[serde(rename = "stagr", default, skip_serializing_if = "Vec::is_empty")]
    pub relevant_reading: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pos: Vec<&'static Pos>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gloss: Vec<Gloss>,
}

#[derive(Debug)]
pub struct Pos(pub &'static str, pub &'static str);

impl<'de> Deserialize<'de> for &Pos {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tag = String::deserialize(deserializer)?;

        map_to_pos(&tag).map_err(de::Error::custom)
    }
}

impl Serialize for Pos {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0)
    }
}

fn map_to_pos(tag: &str) -> Result<&'static Pos, String> {
    #[rustfmt::skip]
    const MAP: [Pos; 92] = [
        Pos("&adj-f;", "noun or verb acting prenominally"),
        Pos("&adj-i;", "adjective (keiyoushi)"),
        Pos("&adj-ix;", "adjective (keiyoushi) - yoi/ii class"),
        Pos("&adj-kari;", "'kari' adjective (archaic)"),
        Pos("&adj-ku;", "'ku' adjective (archaic)"),
        Pos("&adj-na;", "adjectival nouns or quasi-adjectives (keiyodoshi)"),
        Pos("&adj-nari;", "archaic/formal form of na-adjective"),
        Pos("&adj-no;", "nouns which may take the genitive case particle 'no'"),
        Pos("&adj-pn;", "pre-noun adjectival (rentaishi)"),
        Pos("&adj-shiku;", "'shiku' adjective (archaic)"),
        Pos("&adj-t;", "'taru' adjective"),
        Pos("&adv;", "adverb (fukushi)"),
        Pos("&adv-to;", "adverb taking the 'to' particle"),
        Pos("&aux;", "auxiliary"),
        Pos("&aux-adj;", "auxiliary adjective"),
        Pos("&aux-v;", "auxiliary verb"),
        Pos("&conj;", "conjunction"),
        Pos("&cop;", "copula"),
        Pos("&ctr;", "counter"),
        Pos("&exp;", "expressions (phrases, clauses, etc.)"),
        Pos("&int;", "interjection (kandoushi)"),
        Pos("&n;", "noun (common) (futsuumeishi)"),
        Pos("&n-adv;", "adverbial noun (fukushitekimeishi)"),
        Pos("&n-pr;", "proper noun"),
        Pos("&n-pref;", "noun, used as a prefix"),
        Pos("&n-suf;", "noun, used as a suffix"),
        Pos("&n-t;", "noun (temporal) (jisoumeishi)"),
        Pos("&num;", "numeric"),
        Pos("&pn;", "pronoun"),
        Pos("&pref;", "prefix"),
        Pos("&prt;", "particle"),
        Pos("&suf;", "suffix"),
        Pos("&unc;", "unclassified"),
        Pos("&v-unspec;", "verb unspecified"),
        Pos("&v1;", "Ichidan verb"),
        Pos("&v1-s;", "Ichidan verb - kureru special class"),
        Pos("&v2a-s;", "Nidan verb with 'u' ending (archaic)"),
        Pos("&v2b-k;", "Nidan verb (upper class) with 'bu' ending (archaic)"),
        Pos("&v2b-s;", "Nidan verb (lower class) with 'bu' ending (archaic)"),
        Pos("&v2d-k;", "Nidan verb (upper class) with 'dzu' ending (archaic)"),
        Pos("&v2d-s;", "Nidan verb (lower class) with 'dzu' ending (archaic)"),
        Pos("&v2g-k;", "Nidan verb (upper class) with 'gu' ending (archaic)"),
        Pos("&v2g-s;", "Nidan verb (lower class) with 'gu' ending (archaic)"),
        Pos("&v2h-k;", "Nidan verb (upper class) with 'hu/fu' ending (archaic)"),
        Pos("&v2h-s;", "Nidan verb (lower class) with 'hu/fu' ending (archaic)"),
        Pos("&v2k-k;", "Nidan verb (upper class) with 'ku' ending (archaic)"),
        Pos("&v2k-s;", "Nidan verb (lower class) with 'ku' ending (archaic)"),
        Pos("&v2m-k;", "Nidan verb (upper class) with 'mu' ending (archaic)"),
        Pos("&v2m-s;", "Nidan verb (lower class) with 'mu' ending (archaic)"),
        Pos("&v2n-s;", "Nidan verb (lower class) with 'nu' ending (archaic)"),
        Pos("&v2r-k;", "Nidan verb (upper class) with 'ru' ending (archaic)"),
        Pos("&v2r-s;", "Nidan verb (lower class) with 'ru' ending (archaic)"),
        Pos("&v2s-s;", "Nidan verb (lower class) with 'su' ending (archaic)"),
        Pos("&v2t-k;", "Nidan verb (upper class) with 'tsu' ending (archaic)"),
        Pos("&v2t-s;", "Nidan verb (lower class) with 'tsu' ending (archaic)"),
        Pos("&v2w-s;", "Nidan verb (lower class) with 'u' ending and 'we' conjugation (archaic)"),
        Pos("&v2y-k;", "Nidan verb (upper class) with 'yu' ending (archaic)"),
        Pos("&v2y-s;", "Nidan verb (lower class) with 'yu' ending (archaic)"),
        Pos("&v2z-s;", "Nidan verb (lower class) with 'zu' ending (archaic)"),
        Pos("&v4b;", "Yodan verb with 'bu' ending (archaic)"),
        Pos("&v4g;", "Yodan verb with 'gu' ending (archaic)"),
        Pos("&v4h;", "Yodan verb with 'hu/fu' ending (archaic)"),
        Pos("&v4k;", "Yodan verb with 'ku' ending (archaic)"),
        Pos("&v4m;", "Yodan verb with 'mu' ending (archaic)"),
        Pos("&v4n;", "Yodan verb with 'nu' ending (archaic)"),
        Pos("&v4r;", "Yodan verb with 'ru' ending (archaic)"),
        Pos("&v4s;", "Yodan verb with 'su' ending (archaic)"),
        Pos("&v4t;", "Yodan verb with 'tsu' ending (archaic)"),
        Pos("&v5aru;", "Godan verb - -aru special class"),
        Pos("&v5b;", "Godan verb with 'bu' ending"),
        Pos("&v5g;", "Godan verb with 'gu' ending"),
        Pos("&v5k;", "Godan verb with 'ku' ending"),
        Pos("&v5k-s;", "Godan verb - Iku/Yuku special class"),
        Pos("&v5m;", "Godan verb with 'mu' ending"),
        Pos("&v5n;", "Godan verb with 'nu' ending"),
        Pos("&v5r;", "Godan verb with 'ru' ending"),
        Pos("&v5r-i;", "Godan verb with 'ru' ending (irregular verb)"),
        Pos("&v5s;", "Godan verb with 'su' ending"),
        Pos("&v5t;", "Godan verb with 'tsu' ending"),
        Pos("&v5u;", "Godan verb with 'u' ending"),
        Pos("&v5u-s;", "Godan verb with 'u' ending (special class)"),
        Pos("&v5uru;", "Godan verb - Uru old class verb (old form of Eru)"),
        Pos("&vi;", "intransitive verb"),
        Pos("&vk;", "Kuru verb - special class"),
        Pos("&vn;", "irregular nu verb"),
        Pos("&vr;", "irregular ru verb, plain form ends with -ri"),
        Pos("&vs;", "noun or participle which takes the aux. verb suru"),
        Pos("&vs-c;", "su verb - precursor to the modern suru"),
        Pos("&vs-i;", "suru verb - included"),
        Pos("&vs-s;", "suru verb - special class"),
        Pos("&vt;", "transitive verb"),
        Pos("&vz;", "Ichidan verb - zuru verb (alternative form of -jiru verbs)"),
    ];

    MAP.iter()
        .find(|p| tag == p.0)
        .ok_or(format!("unexpected part-of-speech found: {tag}"))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Gloss {
    pub content: String,
}
