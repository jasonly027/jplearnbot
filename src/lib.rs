use std::{fs::File, io::BufReader, path::Path, process};

pub mod dictionary;

/// Opens a reader for a file.
///
/// # Panics
/// Will panic if there is an error opening the file.
pub fn open_reader(path: &Path) -> BufReader<File> {
    let file = File::open(path).unwrap_or_else(|e| {
        eprintln!("Failed to open file at {}:\n\t{}", path.display(), e);
        process::exit(-1);
    });

    BufReader::new(file)
}
