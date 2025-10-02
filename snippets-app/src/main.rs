use std::env;
use std::fs;
use std::io;
use std::path::Path;

const STORE_FILE: &str = "snippets.json";

#[derive(Clone)]
struct Snippet {
    name: String,
    lang: String,
    code: String,
}

fn load_store() -> io::Result<Vec<Snippet>> {
    if !Path::new(STORE_FILE).exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(STORE_FILE)?;
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("JSON parse error: {e}")))?;
    let arr = v.as_array().cloned().unwrap_or_default();
    let mut out = Vec::new();
    for item in arr {
        let name = item.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let lang = item.get("lang").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let code = item.get("code").and_then(|x| x.as_str()).unwrap_or("").to_string();
        if !name.is_empty() {
            out.push(Snippet { name, lang, code });
        }
    }
    Ok(out)
}

fn save_store(items: &[Snippet]) -> io::Result<()> {
    let arr: Vec<serde_json::Value> = items
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "lang": s.lang,
                "code": s.code
            })
        })
        .collect();
    let text = serde_json::to_string_pretty(&arr)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    fs::write(STORE_FILE, text)
}

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  snippets-app add \"<name>\" \"<lang>\" \"<code>\"");
    eprintln!("  snippets-app list");
    eprintln!("  snippets-app show \"<name>\"");
    eprintln!("  snippets-app delete \"<name>\"");
}

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage();
        return Ok(());
    }

    let cmd = args.remove(0).to_lowercase();
    match cmd.as_str() {
        "add" => {
            if args.len() < 3 {
                eprintln!("need: add \"<name>\" \"<lang>\" \"<code>\"");
                return Ok(());
            }
            let name = args[0].clone();
            let lang = args[1].clone();
            let code = args[2].clone();

            let mut items = load_store()?;
            if let Some(idx) = items.iter()
                .position(|s| s.name.eq_ignore_ascii_case(&name))
            {
                items[idx] = Snippet { name, lang, code };
            } else {
                items.push(Snippet { name, lang, code });
            }
            save_store(&items)?;
            println!("OK");
        }
        "list" => {
            let items = load_store()?;
            if items.is_empty() {
                println!("(порожньо)");
            } else {
                for s in items {
                    println!("- {} [{}] ({} chars)", s.name, s.lang, s.code.len());
                }
            }
        }
        "show" => {
            if args.len() < 1 {
                eprintln!("need: show \"<name>\"");
                return Ok(());
            }
            let name = &args[0];
            let items = load_store()?;
            if let Some(s) = items.iter().find(|x| x.name.eq_ignore_ascii_case(name)) {
                println!("--- {} [{}] ---", s.name, s.lang);
                println!("{}", s.code);
            } else {
                eprintln!("not found: {name}");
            }
        }
        "delete" => {
            if args.len() < 1 {
                eprintln!("need: delete \"<name>\"");
                return Ok(());
            }
            let name = &args[0];
            let mut items = load_store()?;
            let before = items.len();
            items.retain(|x| !x.name.eq_ignore_ascii_case(name));
            if items.len() == before {
                eprintln!("not found: {name}");
            } else {
                save_store(&items)?;
                println!("Deleted");
            }
        }
        _ => {
            print_usage();
        }
    }

    Ok(())
}
