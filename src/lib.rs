use std::{fs::File, io::BufReader, path::Path, process};

pub mod dictionary;

pub fn open_reader(path: &Path) -> BufReader<File> {
    let file = File::open(path).unwrap_or_else(|e| {
        eprintln!("Failed to open file at {}:\n\t{}", path.display(), e);
        process::exit(-1);
    });

    BufReader::new(file)
}
