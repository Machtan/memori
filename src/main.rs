#![feature(unicode)]

extern crate std_unicode;
extern crate hangeul2;
extern crate regex;
#[macro_use]
extern crate lazy_static;

use std::path::Path;
use std::collections::HashMap;
use std::io::{self, Read};
use std::fs::File;
use std_unicode::str::UnicodeStr;
use regex::Regex;

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
                korean = &korean[..korean.len()-1];
            }
            let hanja = caps.get(2).unwrap().as_str().trim();
            let meaning = caps.get(3).unwrap().as_str().trim();
            Note { 
                term: korean.to_string(), 
                meaning: Meaning {
                    text: meaning.to_string(),
                    symbol: if hanja == "" { None } else { Some(hanja.to_string()) }
                }
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
    InvalidNote { file: String, lineno: usize, line: String },
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
        let mut title = Path::new(path).file_name().map(|o| o.to_string_lossy()).unwrap().to_string();
        file.read_to_string(&mut text)?;
        let mut scope = Vocab;
        let mut notes = Vec::new();
        for (lineno, line) in text.lines().enumerate() {
            if line.starts_with("#") {
                if let Some(m) = RE_TITLE.find(&line) {
                    title = (&line[m.end()..]).trim().to_string();
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
        Ok(Source { title: title, contents: notes })
    }
}

#[derive(Debug, Clone)]
pub struct ColMeaning {
    pub text: String,
    pub symbol: Option<String>,
    pub source: u32,
}
impl ColMeaning {
    fn new(meaning: Meaning, source: u32) -> ColMeaning {
        ColMeaning {
            text: meaning.text,
            symbol: meaning.symbol,
            source: source,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Collection {
    contents: HashMap<String, Vec<ColMeaning>>,
    titles: HashMap<u32, String>,
    titles_rev: HashMap<String, u32>,
    next_title_id: u32,
    empty: Vec<ColMeaning>,
}

impl Collection {
    #[inline]
    pub fn new() -> Collection {
        Collection {
            contents: HashMap::new(),
            titles: HashMap::new(),
            titles_rev: HashMap::new(),
            next_title_id: 0,
            empty: Vec::new(),
        }
    }
    
    fn ensure_title(&mut self, title: &str) -> u32 {
        if let Some(&id) = self.titles_rev.get(title) {
            id
        } else {
            let id = self.next_title_id;
            self.next_title_id += 1;
            self.titles.insert(id, title.to_string());
            self.titles_rev.insert(title.to_string(), id);
            id
        }
    }
    
    /// Returns whether the insertion was succesful.
    pub fn insert<'a>(&'a mut self, term: &'a str, meaning: &'a Meaning, source_title: &'a str) -> Option<Insertion<'a>> {
        if ! self.contents.contains_key(term) {
            let id = self.ensure_title(source_title);
            let col_meaning = ColMeaning::new(meaning.clone(), id);
            self.contents.insert(term.to_string(), vec![col_meaning]);
            return None;
        }
        if let Some(meanings) = self.contents.get_mut(term) {
            for col_meaning in meanings {
                if col_meaning.text == meaning.text {
                    if let Some(ref sym) = meaning.symbol {
                        if col_meaning.symbol.is_none() {
                            col_meaning.symbol = Some(sym.clone());
                        }
                    }
                    return None;
                }
            }
        }
        Some(Insertion::new(self, term, meaning, source_title))
    }
    
    // Returns the meanings associated with the given term.
    #[inline]
    pub fn meanings(&self, term: &str) -> &Vec<ColMeaning> {
        if let Some(meanings) = self.contents.get(term) {
            meanings
        } else {
            &self.empty
        }
    }
}

#[derive(Debug)]
#[must_use]
pub struct Insertion<'a> {
    collection: &'a mut Collection,
    term: &'a str,
    meaning: &'a Meaning,
    source_title: &'a str,
}
impl<'a> Insertion<'a> {
    #[inline]
    fn new(collection: &'a mut Collection, term: &'a str, meaning: &'a Meaning, source_title: &'a str) -> Insertion<'a> {
        Insertion {
            collection: collection,
            term: term,
            meaning: meaning,
            source_title: source_title,
        }
    }
    
    #[inline]
    pub fn meanings(&self) -> &Vec<ColMeaning> {
        self.collection.meanings(self.term)
    }
    
    #[inline]
    pub fn title(&self, meaning: &ColMeaning) -> Option<&String> {
        self.collection.titles.get(&meaning.source)
    }
    
    pub fn reject(self) {}
    
    pub fn insert_new(mut self) {
        let id = self.collection.ensure_title(self.source_title);
        let meaning = ColMeaning::new(self.meaning.clone(), id);
        self.collection.contents.get_mut(self.term).unwrap().push(meaning);
    }
    
    pub fn replace_existing(mut self, index: usize) -> Result<(), Self> {
        if index >= self.meanings().len() {
            Err(self)
        } else {
            let id = self.collection.ensure_title(self.source_title);
            let mut meanings = self.collection.contents.get_mut(self.term).unwrap();
            let mut meaning = ColMeaning::new(self.meaning.clone(), id);
            meaning.symbol = meaning.symbol.or_else(|| meanings[index].symbol.clone());
            meanings[index] = meaning;
            Ok(())
        }
    }
    
    pub fn update_existing(mut self, index: usize, new_text: String) -> Result<(), Self> {
        if index >= self.meanings().len() {
            Err(self)
        } else {
            let id = self.collection.ensure_title(self.source_title);
            let mut meanings = self.collection.contents.get_mut(self.term).unwrap();
            let sym = self.meaning.symbol.clone().or_else(|| meanings[index].symbol.clone());
            let meaning = ColMeaning::new(Meaning { text: new_text, symbol: sym}, id);
            meanings[index] = meaning;
            Ok(())
        }
    }
}


fn main() {
    let source_path = "../../../Desktop/memori/notes/lms_v1c2.txt";
    let source = Source::load(source_path).expect("could not read source");
    println!("Title: {}", source.title);
    for note in &source.contents {
        println!("- {:?}", note);
    }
    
    let mut collection = Collection::new();
    for note in &source.contents {
        if let Some(insertion) = collection.insert(&note.term, &note.meaning, &source.title) {
            println!("Decision time! ({}: {})", &note.term, note.meaning.text);
            for (i, meaning) in insertion.meanings().iter().enumerate() {
                println!("{}) {} ['{}']", i+1, meaning.text, insertion.title(meaning).unwrap());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use self::super::RE_VOCAB;
    
    fn test_re(line: &str, e_korean: &str, e_meaning: &str, e_hanja: Option<&str>) {
        if let Some(caps) = RE_VOCAB.captures(line) {
            let mut korean = caps.get(1).unwrap().as_str().trim();
            if korean.ends_with(":") {
                korean = &korean[..korean.len()-1];
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
