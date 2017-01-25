pub struct SplitWhitespaceIndices<'a> {
    source: &'a str,
    start: usize,
    needs_final_split: bool,
}
impl<'a> Iterator for SplitWhitespaceIndices<'a> {
    type Item = (usize, &'a str);
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.source.len() {
            if self.needs_final_split {
                self.needs_final_split = false;
                return Some((self.source.len(), ""));
            } else {
                return None;
            }
        }
        self.needs_final_split = false; // Assume false by default
        let mut chars = (&self.source[self.start..]).char_indices();
        let mut trim_start = None;
        while let Some((offset, ch)) = chars.next() {
            if let Some(end) = trim_start {
                if ! ch.is_whitespace() {
                    let slice = &self.source[self.start..end];
                    let start = self.start;
                    self.start += offset;
                    return Some((start, slice));
                }
            } else  {
                if ch.is_whitespace() {
                    trim_start = Some(self.start + offset);
                    self.needs_final_split = true; // If space is found, split!
                }
            }
        }
        let slice = if let Some(end) = trim_start {
            &self.source[self.start..end]
        } else {
            &self.source[self.start..]
        };
        let start = self.start;
        self.start = self.source.len();
        Some((start, slice))
    }
}

pub fn split_whitespace_indices(text: &str) -> SplitWhitespaceIndices {
    SplitWhitespaceIndices {
        source: text,
        start: 0,
        needs_final_split: false,
    }
}

/*
    let text = "the quick brown fox jumped over the lazy dog";
    for (i, word) in split_whitespace_indices(text) {
        let end = i + word.len();
        let before = &text[..end];
        let after = &text[end..];
        println!("{}: {}", i, word);
        println!("{} | {}", before, after);
    }
*/

#[cfg(test)]
mod tests {
    use self::super::split_whitespace_indices;
    
    #[test]
    fn space() {
        let space = " ";
        let mut s = split_whitespace_indices(space);
        assert_eq!(Some((0, "")), s.next());
        assert_eq!(Some((1, "")), s.next());
        assert_eq!(None, s.next());
    }
    
    #[test]
    fn single() {
        let single = "derp";
        let mut s = split_whitespace_indices(single);
        assert_eq!(Some((0, "derp")), s.next());
        assert_eq!(None, s.next());
    }
        
    #[test]
    fn trailing() {
        let trailing = "hello ";
        let mut s = split_whitespace_indices(trailing);
        assert_eq!(Some((0, "hello")), s.next());
        assert_eq!(Some((6, "")), s.next());
        assert_eq!(None, s.next());
    }
    
    #[test]
    fn leading() {
        let leading = " hello";
        let mut s = split_whitespace_indices(leading);
        assert_eq!(Some((0, "")), s.next());
        assert_eq!(Some((1, "hello")), s.next());
        assert_eq!(None, s.next());
    }
    
    #[test]
    fn both() {
        let both = " hello ";
        let mut s = split_whitespace_indices(both);
        assert_eq!(Some((0, "")), s.next());
        assert_eq!(Some((1, "hello")), s.next());
        assert_eq!(Some((7, "")), s.next());
        assert_eq!(None, s.next());
    }
    
    #[test]
    fn two() {
        let two = "hello world";
        let mut s = split_whitespace_indices(two);
        assert_eq!(Some((0, "hello")), s.next());
        assert_eq!(Some((6, "world")), s.next());
        assert_eq!(None, s.next());
    }
    
    #[test]
    fn multiple() {
        let multiple = "  hello   world    ";
        let mut s = split_whitespace_indices(multiple);
        assert_eq!(Some((0, "")), s.next());
        assert_eq!(Some((2, "hello")), s.next());
        assert_eq!(Some((10, "world")), s.next());
        assert_eq!(Some((19, "")), s.next());
        assert_eq!(None, s.next());
    }
}