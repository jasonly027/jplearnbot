use std::{cell::RefCell, collections::HashMap, io::BufRead, path::Path, rc::Rc};

use jplearnbot::{dictionary::DictEntry, open_reader};

pub fn dict(file: &Path) -> HashMap<String, Vec<Rc<RefCell<DictEntry>>>> {
    let entries: Vec<_> = entries(file)
        .into_iter()
        .map(RefCell::new)
        .map(Rc::new)
        .collect();

    let mut map: HashMap<String, Vec<_>> = HashMap::new();
    for entry in entries {
        for reading in &entry.borrow().readings {
            map.entry(reading.hiragana.clone())
                .or_default()
                .push(entry.clone());
        }
    }

    map
}

fn entries(file: &Path) -> Vec<DictEntry> {
    let reader = open_reader(file);

    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line.unwrap_or_else(|e| panic!("Invalid byte read in dfile:\n{e}"));

        let entry =
            serde_json::from_str(&line).unwrap_or_else(|e| panic!("JSON Parse error:\n{e}"));

        entries.push(entry);
    }

    entries
}
