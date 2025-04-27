use std::{
    cell::RefCell,
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
    rc::Rc,
};

use jplearnbot::dictionary::DictEntry;

use crate::{
    dict,
    jlpt::{self, JlptEntry, JlptNLevel},
};

pub fn run(dir: &Path, overwrite: bool) {
    let entries = dict_entries(dir);
    let mut writer = writer(dir, overwrite);

    for entry in entries {
        let mut str = serde_json::to_string(&*entry.borrow()).unwrap();
        str.push('\n');

        writer
            .write_all(str.as_bytes())
            .unwrap_or_else(|e| panic!("Failed to write to output:\n{e}"));
    }

    writer.flush().expect("Failed to flush to output");
}

fn dict_entries(dir: &Path) -> Vec<Rc<RefCell<DictEntry>>> {
    let dict = annotated_dict(dir);

    let mut set: HashMap<u32, _> = HashMap::new();
    for entries in dict.into_values() {
        for entry in entries {
            if entry.borrow().is_annotated() {
                let id = entry.borrow().id;
                set.insert(id, entry);
            }
        }
    }

    set.into_values().collect()
}

fn annotated_dict(dir: &Path) -> HashMap<String, Vec<Rc<RefCell<DictEntry>>>> {
    let dict = dict::dict(&dir.join("jmdict.jsonl"));

    for pool in [
        JlptNLevel::One,
        JlptNLevel::Two,
        JlptNLevel::Three,
        JlptNLevel::Four,
    ]
    .into_iter()
    .map(|lvl| jlpt::pool(dir, lvl))
    {
        for JlptEntry {
            hiragana,
            kanji,
            level,
        } in &pool
        {
            let Some(matches) = dict.get(hiragana) else {
                continue;
            };

            // No definition ambiguity, mutate the exact match
            if matches.len() == 1 {
                matches[0].borrow_mut().set_level(hiragana, (*level).into());
                continue;
            }

            // Entry has kanji, mutate the only match with the same kanji, if it exists
            if let Some(kanji) = kanji {
                let matches: Vec<_> = matches
                    .iter()
                    .filter(|m| m.borrow().kanjis.iter().any(|k| k.kanji == *kanji))
                    .collect();

                if matches.len() == 1 {
                    matches[0].borrow_mut().set_level(hiragana, (*level).into());
                }

                continue;
            }

            // Entry has no kanji, mutate the only match without kanji too, if it exists
            let matches: Vec<_> = matches
                .iter()
                .filter(|m| m.borrow().kanjis.is_empty())
                .collect();

            if matches.len() == 1 {
                matches[0].borrow_mut().set_level(hiragana, (*level).into());
            }
        }
    }

    dict
}

fn writer(dir: &Path, overwrite: bool) -> BufWriter<File> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(!overwrite)
        .create(overwrite)
        .truncate(overwrite)
        .open(dir.join("dictionary.jsonl"))
        .unwrap_or_else(|e| panic!("Error writing output:\n{e}"));

    BufWriter::new(file)
}
