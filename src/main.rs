#![feature(unicode)]

extern crate std_unicode;
extern crate hangeul2;

mod split_whitespace;

use std::path::Path;
use std::collections::HashMap;
use std::io::{self, Read};
use std::fs::File;
use std_unicode::str::UnicodeStr;
use split_whitespace::split_whitespace_indices;

use hangeul2::is_hangeul;

#[derive(Debug, Clone)]
pub struct Meaning {
    pub reading: String,
    pub symbol: Option<String>, 
}

#[derive(Debug, Clone)]
pub struct Note {
    pub foreign: String,
    pub meaning: Meaning,
}

#[derive(Debug, Clone)]
pub struct Source {
    pub title: String,
    pub contents: Vec<Note>,
}

#[derive(Debug)]
pub enum SourceLoadError {
    Io(io::Error),
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
}

impl Source {
    pub fn load(path: &str) -> Result<Source, SourceLoadError> {
        use self::SourceScope::*;
        let mut file = File::open(path)?;
        let mut text = String::new();
        let mut title = Path::new(path).file_name().map(|o| o.to_string_lossy()).unwrap().to_string();
        file.read_to_string(&mut text)?;
        let mut scope = Vocab;
        for line in text.lines() {
            if line.starts_with("#") {
                if let Some(index) = line.find("title") {
                    title = (&line[index..]).split(" ").last().unwrap_or("").to_string();
                } else if let Some(_) = line.find("read") {
                    scope = ReadingExample
                }
                continue;
            } else if line.is_whitespace() {
                if let ReadingVocab = scope {
                    scope = ReadingExample;
                }
                continue;
            } else {
                match scope {
                    Vocab | ReadingVocab => {
                    
                    }
                    ReadingExample => {
                        scope = ReadingVocab;
                        // Ignore the example line
                    }
                }
            }
        }
        unimplemented!();
    }
}

#[derive(Debug, Clone)]
pub struct ColMeaning {
    pub meaning: Meaning,
    pub source: u32,
}

#[derive(Debug, Clone)]
pub struct Collection {
    pub contents: HashMap<String, Vec<ColMeaning>>,
    pub titles: HashMap<u32, String>,
    next_title: u32,
}


fn main() {
    let source_path = "~/Desktop/memori/notes/lms_v1c2.txt";
    println!("Hello, world!");
    //let mut source = Source::load(source_path).expect("could not read source");
    
}
