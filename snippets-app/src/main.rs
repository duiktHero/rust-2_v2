use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use serde::{Deserialize, Serialize};

const STORE_FILE: &str = "snippets.json";

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Snippet {
    name: String,
    lang: String,
    code: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
struct Store {
    snippets: Vec<Snippet>,
}

impl Store {
    fn load_store() -> std::io::Result<Self> {
        if !Path::new(STORE_FILE).exists() {
            return Ok(Self::default());
        }
        let file = File::open(STORE_FILE)?;
        let reader = BufReader::new(file);
        let snippets: Vec<Snippet> = serde_json::from_reader(reader).unwrap_or_default();
        Ok(Self { snippets })
    }

    fn save_store(&self) -> std::io::Result<()> {
        let file = File::create(STORE_FILE)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self.snippets)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

fn main() -> std::io::Result<()> {
    let mut store = Store::load_store()?;
    println!("Loaded {} snippet(s).", store.snippets.len());

    if store.snippets.is_empty() {
        store.snippets.push(Snippet {
            name: "hello".into(),
            lang: "rust".into(),
            code: r#"fn main() { println!("hi"); }"#.into(),
        });
    }
    store.save_store()?;

    println!("Saved {} snippet(s) to {}.", store.snippets.len(), STORE_FILE);
    Ok(())
}
