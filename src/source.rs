use std_unicode::str::UnicodeStr;
use regex::Regex;
use std::path::Path;
use std::io::{self, Read};
use std::fs::File;

/*
Where to find recognized unicode class names:
regex/regex-syntax/src/unicode.rs:4348
UNICODE_CLASSES
*/

lazy_static! {
    static ref RE_VOCAB: Regex = Regex::new(
        r"((?:(?:[\-\(\)/~NIAV]|\p{Hangul})+(?:\s|:)+)+)((?:(?:\p{Han}|\s)+)?)\s*(.+)"
        ).unwrap();
}

#[derive(Debug, Clone)]
pub struct Meaning {
    pub text: String,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub term: String,
    pub meaning: Meaning,
}
impl Note {
    pub fn from_line(line: &str) -> Option<Note> {
        RE_VOCAB.captures(line).map(|caps| {
            let mut korean = caps.get(1).unwrap().as_str().trim();
            if korean.ends_with(":") {
                korean = &korean[..korean.len() - 1];
            }
            let hanja = caps.get(2).unwrap().as_str().trim();
            let meaning = caps.get(3).unwrap().as_str().trim();
            Note {
                term: korean.to_string(),
                meaning: Meaning {
                    text: meaning.to_string(),
                    symbol: if hanja == "" {
                        None
                    } else {
                        Some(hanja.to_string())
                    },
                },
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct Source {
    pub title: String,
    pub contents: Vec<Note>,
}

#[derive(Debug)]
pub enum SourceLoadError {
    Io(io::Error),
    InvalidNote {
        file: String,
        lineno: usize,
        line: String,
    },
}

impl From<io::Error> for SourceLoadError {
    fn from(err: io::Error) -> SourceLoadError {
        SourceLoadError::Io(err)
    }
}

enum SourceScope {
    Vocab,
    ReadingExample,
    ReadingVocab,
    Title,
}

lazy_static! {
    static ref RE_TITLE: Regex = Regex::new(r"# ?[tT]itle:?").unwrap();
    static ref RE_READING: Regex = Regex::new(r"# ?[rR]ead").unwrap();
    static ref RE_VOCABULARY: Regex = Regex::new(r"# ?[vV]ocab").unwrap();
}

impl Source {
    pub fn load(path: &str) -> Result<Source, SourceLoadError> {
        use self::SourceLoadError::*;
        use self::SourceScope::*;
        let mut file = File::open(path)?;
        let mut text = String::new();
        let mut title =
            Path::new(path).file_name().map(|o| o.to_string_lossy()).unwrap().to_string();
        file.read_to_string(&mut text)?;
        let mut scope = Vocab;
        let mut notes = Vec::new();
        for (lineno, line) in text.lines().enumerate() {
            if line.starts_with("#") {
                if let Some(m) = RE_TITLE.find(&line) {
                    let rem = (&line[m.end()..]).trim();
                    if rem != "" {
                        title = rem.to_string();
                    } else {
                        scope = Title;
                    }
                } else if RE_READING.find(&line).is_some() {
                    scope = ReadingExample;
                } else if RE_VOCABULARY.find(&line).is_some() {
                    scope = Vocab;
                }
                continue;
            } else if line.is_whitespace() {
                if let ReadingVocab = scope {
                    scope = ReadingExample;
                }
                continue;
            } else {
                match scope {
                    Title => {
                        if line.is_whitespace() {
                            continue;
                        } else {
                            title = line.trim().to_string();
                            scope = Vocab;
                        }
                    }
                    Vocab | ReadingVocab => {
                        if let Some(note) = Note::from_line(&line) {
                            notes.push(note);
                        } else {
                            return Err(InvalidNote {
                                file: path.to_string(),
                                lineno: lineno,
                                line: line.to_string(),
                            });
                        }
                    }
                    ReadingExample => {
                        scope = ReadingVocab;
                        // Ignore the example line
                    }
                }
            }
        }
        Ok(Source {
            title: title,
            contents: notes,
        })
    }
}
