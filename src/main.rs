#![feature(unicode)]

extern crate std_unicode;
extern crate hangeul2;
extern crate regex;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod source;
mod history;
mod collection;

use std::path::Path;
use std::io::{self, Read, Write};
use std::fs::File;
use std::env;
use std::process::{self, Command};
use std::error::Error;
use source::{Source, Note};
use collection::Collection;
use history::History;

fn print_usage(errno: i32) -> Result<(), i32> {
    println!("Usage: memori integrate <collection.json> <source.txt> [<source.txt> ...]");
    if errno != 0 { Err(errno) } else { Ok(()) }
}

#[cfg(target_os = "macos")]
fn open_in_dictionary(word: &str) {
    let url = format!("dict://{}", word.replace(" ", "%20"));
    let mut cmd = Command::new("open");
    let _ = cmd.arg(&url).spawn();
}

#[cfg(not(target_os = "macos"))]
fn open_in_dictionary(word: &str) {}

fn prompt_answer<T, F: FnMut(&str) -> Option<T>>(initial: Option<&str>,
                                                 mut convertor: F)
                                                 -> Result<T, io::Error> {
    if let Some(initial) = initial {
        if let Some(res) = convertor(initial) {
            return Ok(res);
        }
    }
    let mut input = String::new();
    print!("> ");
    loop {
        io::stdout().flush()?;
        io::stdin().read_line(&mut input)?;
        input = input.trim().to_string();
        if let Some(res) = convertor(&input) {
            return Ok(res);
        } else {
            print!("! ");
        }
    }
}

#[derive(Debug)]
enum Decision {
    Reject,
    Add(String),
    Update(usize, String),
}

fn prompt_index(initial: &str, n_meanings: usize) -> Result<usize, i32> {
    prompt_answer(if initial == "" { None } else { Some(initial) },
                  |inp| if let Ok(index) = inp.parse::<usize>() {
                      if index < n_meanings {
                          Some(index)
                      } else {
                          println!("Index too big (>= {})", n_meanings);
                          None
                      }
                  } else {
                      None
                  })
        .map_err(|_| 6)
}

fn prompt_decision(collection: &Collection,
                   note: &Note,
                   source_title: &String)
                   -> Result<Decision, i32> {
    println!("New meaning found: ( from '{}' )", source_title);
    println!("{} | {}", note.term, note.meaning.text);
    println!("Existing meanings:");
    let n_meanings = collection.meanings(&note.term).len();
    for (i, meaning) in collection.meanings(&note.term).iter().enumerate() {
        println!("{}) {} ['{}']",
                 i,
                 meaning.text,
                 collection.title(meaning.source).unwrap());
    }
    println!("[a]dd [r]eplace [u]pdate [i]gnore");
    print!("> ");
    loop {
        let mut input = String::new();
        if let Err(err) = io::stdin().read_line(&mut input) {
            println!("Error reading from stdin: '{}'", err.description());
            return Err(7);
        }
        let answer = input.trim();
        let (cmd, rem) = if let Some(index) = answer.find(" ") {
            (&answer[..index], &answer[index + 1..])
        } else {
            (answer, "")
        };
        match cmd {
            "a" => {
                return Ok(Decision::Add(note.meaning.text.clone()));
            }
            "r" => {
                println!("Please choose the index to replace ({} : {}):",
                         0,
                         n_meanings - 1);
                let index = prompt_index(rem, n_meanings)?;
                return Ok(Decision::Update(index, note.meaning.text.clone()));
            }
            "u" => {
                println!("Please choose the index to update ({} : {}):",
                         0,
                         n_meanings - 1);
                let (indextext, updatetext) = if let Some(index) = rem.find(" ") {
                    (&rem[..index], Some(&rem[index..]))
                } else {
                    (rem, None)
                };
                let index = prompt_index(indextext, n_meanings)?;
                let text = prompt_answer(updatetext, |inp| if inp != "" {
                        Some(inp.to_string())
                    } else {
                        None
                    }).map_err(|_| 6)?;
                return Ok(Decision::Update(index, text));
            }
            "i" => {
                return Ok(Decision::Reject);
            }
            _ => {
                println!("Unrecognized command: '{}'", cmd);
            }
        }
    }
}

fn load_collection(colpath: &str) -> Result<Collection, i32> {
    let cpath = Path::new(colpath);
    Ok(if !cpath.exists() {
        Collection::new()
    } else {
        let mut file = match File::open(&cpath) {
            Ok(file) => file,
            Err(err) => {
                println!("Could not open collection file : '{}'", err.description());
                return Err(2);
            }
        };
        let mut json = String::new();
        if let Err(err) = file.read_to_string(&mut json) {
            println!("Could not read collection file: '{}'", err.description());
            return Err(3);
        }
        match Collection::from_json(&json) {
            Ok(col) => col,
            Err(err) => {
                println!("Could not parse collection JSON: '{}'", err.description());
                return Err(4);
            }
        }
    })
}

fn load_history(hispath: &str) -> Result<History, i32> {
    let path = Path::new(hispath);
    Ok(if !path.exists() {
        History::new()
    } else {
        let mut file = File::open(&path).expect("Could not open file");
        let mut json = String::new();
        file.read_to_string(&mut json).expect("Could not read file");
        History::from_json(&json).expect("Could not parse JSON to struct")
    })
}

fn save_history(history: &History, hispath: &str) -> Result<(), i32> {
    let serialized = serde_json::to_string(&history).unwrap();
    let mut outfile = match File::create(&hispath) {
        Ok(f) => f,
        Err(err) => {
            println!("Could not open history for writing ('{}'): '{}'",
                     hispath,
                     err.description());
            return Err(6);
        }
    };
    if let Err(err) = outfile.write_all(serialized.as_bytes()) {
        println!("Could not write to history file ('{}'): '{}'",
                 hispath,
                 err.description());
        return Err(6);
    }
    println!("Saved history, woohoo!");
    Ok(())
}

fn save_collection(collection: &Collection, colpath: &str) -> Result<(), i32> {
    let serialized = serde_json::to_string(&collection).unwrap();
    let mut outfile = match File::create(&colpath) {
        Ok(f) => f,
        Err(err) => {
            println!("Could not open collection for writing ('{}'): '{}'",
                     colpath,
                     err.description());
            return Err(6);
        }
    };
    if let Err(err) = outfile.write_all(serialized.as_bytes()) {
        println!("Could not write to collection file ('{}'): '{}'",
                 colpath,
                 err.description());
        return Err(6);
    }
    println!("Saved collection, yay!");
    Ok(())
}




fn run() -> Result<(), i32> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 1 {
        print_usage(1)?;
    }
    let ref cmd = args[0];
    match cmd.as_str() {
        "integrate" => {
            if args.len() < 3 {
                print_usage(1)?;
            }
            let colpath = &args[1];
            let hispath = &args[2];

            let source_paths = &args[3..];

            let mut collection = load_collection(colpath)?;
            let mut history = load_history(hispath)?;

            for source_path in source_paths {
                let source = match Source::load(source_path) {
                    Ok(s) => s,
                    Err(err) => {
                        println!("Could not read source at {}: {:?}", source_path, err);
                        return Err(5);
                    }
                };
                for note in &source.contents {
                    // If not note handled in history
                    if history.contains(&note.term, &note.meaning.text) {
                        println!("- Skipping ({} | {})", &note.term, &note.meaning.text);
                        continue;
                    }
                    if !collection.contains(&note.term, &note.meaning.text) {
                        println!("Adding ({} | {})!", &note.term, &note.meaning.text);
                        history.insert(note.term.clone(), note.meaning.text.clone());
                        collection.add_meaning(note.term.clone(), note.meaning.clone(), &source.title);
                        continue;
                    }
                    open_in_dictionary(&note.term);
                    match prompt_decision(&collection, note, &source.title) {
                        Ok(Decision::Reject) => {
                            println!("Rejected!");
                            history.insert(note.term.clone(), note.meaning.text.clone());
                        }
                        Ok(Decision::Add(meaning)) => {
                            println!("Adding new!");
                            history.insert(note.term.clone(), meaning);
                            collection.add_meaning(note.term.clone(),
                                                   note.meaning.clone(),
                                                   &source.title);
                        }
                        Ok(Decision::Update(index, meaning)) => {
                            println!("Updating [{}] => {}", index, meaning);
                            history.insert(note.term.clone(), note.meaning.text.clone());
                            collection.replace_meaning(&note.term, index, 
                                meaning, None, &source.title).expect("INVARIANT!");
                        }
                        Err(err) => {
                            // Save and quit
                            save_collection(&collection, colpath)?;
                            save_history(&history, hispath)?;
                            Err(err)?;
                        }
                    }
                }
            }

            save_collection(&mut collection, colpath)?;
            save_history(&history, hispath)?;
        }
        "lookup" => {
            if args.len() < 3 {
                print_usage(1)?;
            }
            let colpath = &args[1];
            let term = &args[2];
            let collection = load_collection(colpath)?;
            let ref meanings = collection.meanings(term);
            if meanings.len() == 0 {
                println!("No meanings found");
            } else {
                for (i, meaning) in meanings.iter().enumerate() {
                    println!("{}) {} ['{}']",
                             i,
                             meaning.text,
                             collection.title(meaning.source).unwrap());
                }
            }
        }
        _ => {
            println!("Unsupported command");
            print_usage(1)?;
        }
    }
    Ok(())
}

fn main() {
    if let Err(errno) = run() {
        process::exit(errno);
    }
}

#[cfg(test)]
mod tests {
    use self::super::RE_VOCAB;

    fn test_re(line: &str, e_korean: &str, e_meaning: &str, e_hanja: Option<&str>) {
        if let Some(caps) = RE_VOCAB.captures(line) {
            let mut korean = caps.get(1).unwrap().as_str().trim();
            if korean.ends_with(":") {
                korean = &korean[..korean.len() - 1];
            }
            let hanja = caps.get(2).unwrap().as_str().trim();
            let hanja = if hanja != "" { Some(hanja) } else { None };
            let meaning = caps.get(3).unwrap().as_str().trim();
            assert_eq!(e_korean, korean);
            assert_eq!(e_meaning, meaning);
            assert_eq!(e_hanja, hanja);
            //println!("{} | {} | {}", korean, hanja, meaning);
        } else {
            panic!("Could not read line: '{}'", line);
        }
    }

    #[test]
    fn vocab_re_1() {
        let l1 = "적 tidspunkt (situation, oplevelse)";
        test_re(l1, "적", "tidspunkt (situation, oplevelse)", None);
    }

    #[test]
    fn vocab_re_2() {
        let l2 = "AV~(으)ㄴ/는/(으)ㄹ 데 sted";
        test_re(l2, "AV~(으)ㄴ/는/(으)ㄹ 데", "sted", None);
    }

    #[test]
    fn vocab_re_3() {
        let l3 = "~복: 服 ~tøj";
        test_re(l3, "~복", "~tøj", Some("服"));
    }

    #[test]
    fn vocab_re_4() {
        let l4 = "A~(으)ㄴ가요 blød interrogativ";
        test_re(l4, "A~(으)ㄴ가요", "blød interrogativ", None);
    }

    #[test]
    fn vocab_re_5() {
        let l5 = "가상 현실 virtual reality";
        test_re(l5, "가상 현실", "virtual reality", None);
    }
}
