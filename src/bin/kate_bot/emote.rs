use std::env;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref WOW: String = env::var("EMOTE_WOW").unwrap();
    pub static ref FUBU_LAUGH: String = env::var("EMOTE_FUBU_LAUGH").unwrap();
    pub static ref SCRAJJ: String = env::var("EMOTE_SCRAJJ").unwrap();
    pub static ref ANW: String = env::var("EMOTE_ANW").unwrap();
    pub static ref WAT: String = env::var("EMOTE_WAT").unwrap();
}
