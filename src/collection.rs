use std::collections::HashMap;
use source::Meaning;
use serde_json;

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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

    pub fn from_json(json: &str) -> serde_json::Result<Collection> {
        serde_json::from_str(json)
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

    pub fn title(&self, id: u32) -> Option<&String> {
        self.titles.get(&id)
    }

    pub fn add_meaning(&mut self, term: String, meaning: Meaning, source_title: &str) {
        let id = self.ensure_title(source_title);
        let meaning = ColMeaning::new(meaning, id);
        self.contents.entry(term).or_insert(Vec::new()).push(meaning);
    }

    pub fn replace_meaning(&mut self,
                           term: &str,
                           index: usize,
                           text: String,
                           symbol: Option<String>,
                           source_title: &str)
                           -> Result<(), String> {
        let n_meanings = self.contents.get(term).map(|m| m.len()).unwrap_or(0);
        if n_meanings == 0 {
            Err(format!("No meanings found for term '{}'", term))
        } else if index >= n_meanings {
            Err(format!("Invalid index: {} >= {}.", index, n_meanings))
        } else {
            let id = self.ensure_title(source_title);
            let ref mut meanings = self.contents.get_mut(term).unwrap();
            let ref mut colmeaning = meanings[index];
            colmeaning.text = text;
            if colmeaning.symbol.is_none() {
                colmeaning.symbol = symbol;
            }
            colmeaning.source = id;
            Ok(())
        }
    }

    /// Returns whether a definition for the given term is contained.
    #[inline]
    pub fn contains(&self, term: &str, meaning: &str) -> bool {
        self.contents.get(term).map(|m| m.iter().any(|cm| &cm.text == meaning)).unwrap_or(false)
    }

    /// Returns the meanings associated with the given term.
    #[inline]
    pub fn meanings(&self, term: &str) -> &Vec<ColMeaning> {
        if let Some(meanings) = self.contents.get(term) {
            meanings
        } else {
            &self.empty
        }
    }
}
