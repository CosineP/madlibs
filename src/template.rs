// Deals with the madlibs logic: templates, POS, filling in, etc

use senna::senna::*;
use stopwords::Stopwords;

use regex::Regex;
use rand::Rng;
use std::collections::HashSet;
use std::collections::HashMap;

use pos::*;

use sanitize_all;

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Token {
    pub text: Option<String>,
    pub is_placeholder: bool,
    pub pos: Option<POS>,
}

impl Token {
    pub fn new_text(text: String) -> Self {
        Token {
            text: Some(text),
            is_placeholder: false,
            pos: None,
        }
    }
    pub fn new_str(text: &str) -> Self {
        Token::new_text(text.to_string())
    }
    pub fn new_pos(pos: POS) -> Self {
        Token {
            text: None,
            is_placeholder: true,
            pos: Some(pos),
        }
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_placeholder {
            if let Some(text) = &self.text {
                write!(f, "{}[{}]", text, pos_to_str(&self.pos.unwrap()))
            } else {
                write!(f, "{}", pos_to_str(&self.pos.unwrap()))
            }
        } else {
            write!(f, "{}", self.text.as_ref().unwrap())
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Template {
    pub title: Option<String>,
    pub body: Vec<Token>,
}

fn sanitize_source(status: &str) -> String {
    let status = sanitize_all(status);
    // Remove URLs that make https [plural] and // [noun]
    let re = Regex::new(r"http\S+").unwrap();
    let status = re.replace_all(&status, "");
    // Remove any apostrophes, because they fuck up rust-senna
    let re = Regex::new(r"'").unwrap();
    let status = re.replace_all(&status, "");
    status.to_string()
}
fn sanitize_template(status: &str) -> String {
    let status = sanitize_all(status);
    // Remove the @mention at our own account
    let re = Regex::new(r"@<?\w*>?madlibs@?\w*").unwrap();
    let status = re.replace_all(&status, "");
    status.to_string()
}

fn label_status(status: &str) -> Vec<Token> {
    let status = sanitize_source(status);

    let stops: HashSet<_> = stopwords::Spark::stopwords(stopwords::Language::English).unwrap().iter().collect();

    let mut labelled = Vec::new();
    // The data directory I just added from rust-senna submodule because I'm lazy
    let mut senna = Senna::new("rust-senna/senna/".to_string());
    let options = SennaParseOptions {
        psg: false,
        pos: true,
    };
    let sen = senna.parse(&status, options);
    for word in sen.get_words() {
        if !stops.contains(&word.get_string().clone()) {
            let sen_pos = word.get_pos();
            let token = Token {
                text: Some(word.get_string().to_string()),
                is_placeholder: true,
                // CHECK: None here might cause disaster
                pos: senna_to_pos(sen_pos),
            };
            labelled.push(token);
        }
    }
    labelled
}

impl Template {

    fn collect(&self) -> String {
        let body = self.body.iter().fold(String::new(), |s, token| {
            format!("{}{}", s, token.text.as_ref().unwrap())
        });
        match &self.title {
            Some(title) => format!("{}:\n{}", title, body),
            None => body,
        }
    }

    // returns Some(collect()ed String) if done, otherwise None
    pub fn check_done(&self) -> Option<String> {
        // If every single one is not a placeholder, we're done!
        if self.body.iter().all(|token| !token.is_placeholder) {
            Some(self.collect())
        } else {
            None
        }
    }

    // returns true if it succeeded, false if no match was found
    pub fn insert_placeholder(&mut self, pos: POS, word: String) -> bool {
        for template_word in &mut self.body {
            // A placeholder matches a word found
            if template_word.is_placeholder && template_word.pos == Some(pos) {
                // We have found a match!
                template_word.text = Some(word);
                template_word.is_placeholder = false;
                return true;
            }
        }
        return false;
    }

    // Modifies self in-line
    // Returns self.check_done()
    // Only fills in one word, exits immediately
    // (i.e. it's made for one word per status)
    pub fn reduce(&mut self, status: &str) -> Option<String> {
        let mut status = label_status(&sanitize_source(&status));
        // Don't just take the first one, because that tends to be boring
        let mut rng = rand::thread_rng();
        rng.shuffle(&mut status);
        for loan_word in status {
            if self.insert_placeholder(loan_word.pos.unwrap(), loan_word.text.unwrap()) {
                break;
            }
        }
        self.check_done()
    }

    pub fn requirements(&self) -> HashMap<POS, usize> {
        let mut rv = HashMap::new();
        for token in &self.body {
            if let Some(pos) = token.pos {
                rv.entry(pos)
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
            }
        }
        rv
    }

    pub fn parse(status: &str) -> Result<Self, ParseError> {
        let status = sanitize_template(status);

        const OPEN: char = '[';
        const CLOSE: char = ']';
        const END_TITLE: char = ':';
        let mut body = Vec::new();
        let mut in_brace = false;
        let mut chunk = String::new();
        let mut title = None;
        for c in status.chars() {
            match c {
                OPEN => {
                    if in_brace {
                        return Err(ParseError::NestedBrackets);
                    }
                    in_brace = true;
                    let token = Token::new_text(chunk);
                    body.push(token);
                    chunk = String::new();
                },
                CLOSE => {
                    if !in_brace {
                        return Err(ParseError::MismatchedBracket);
                    }
                    in_brace = false;
                    let pos = match str_to_pos(&chunk) {
                        Some(p) => p,
                        None => return Err(ParseError::UnknownPOS(chunk)),
                    };
                    let token = Token::new_pos(pos);
                    body.push(token);
                    chunk = String::new();
                },
                // if we already have a title then the colon is just part of it
                // also, if the part before the colon had template vars,
                // it wasn't really a title
                END_TITLE if title.is_none() && body.len() == 0 => {
                    title = Some(chunk);
                    chunk = String::new();
                },
                _ => {
                    chunk.push(c);
                }
            };
        }
        if in_brace {
            return Err(ParseError::MismatchedBracket);
        }
        // Always end with a non-placeholder representing the end, or at least ""
        let token = Token::new_text(chunk);
        body.push(token);
        // If we identified a title, then we didn't end up with template words,
        // actually we just have a non-template literal
        // This is an && but if-let isn't good enough for that yet
        if let Some(t) = &title {
            if let Some(b) = &body[0].text {
                if body.len() == 1 {
                    body[0].text = Some(format!("{}:{}", t, b));
                    title = None;
                }
            }
        }
        Ok(Template {
            title,
            body,
        })
    }

}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    MismatchedBracket,
    NestedBrackets,
    UnknownPOS(String),
}
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use self::ParseError::*;
        match self {
            MismatchedBracket => write!(f, "brackets[] did not match up 1:1"),
            NestedBrackets => write!(f, "nesting [brackets [like this]] is not allowed"),
            UnknownPOS(given) => write!(f, "unknown part of speech {}", given),
        }
    }
}
impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[cfg(test)]
mod test {
    use super::{POS, Token, Template, sanitize_source};
    #[test]
    fn sanity_templates() {
        let got = Template::parse(
            "my [noun] [verbs] all the boys to the yard and [pronoun] like")
            .unwrap();
        let exp = vec![
            Token::new_str("my "),
            Token::new_pos(POS::Noun),
            Token::new_str(" "),
            Token::new_pos(POS::Verbs),
            Token::new_str(" all the boys to the yard and "),
            Token::new_pos(POS::Pronoun),
            Token::new_str(" like"),
        ];
        assert_eq!(got.body, exp);
        assert_eq!(got.title, None);
    }
    #[test]
    fn titles() {
        let got = Template::parse(
            "the bowman: simple [noun]!")
            .unwrap();
        let exp = vec![
            Token::new_str(" simple "),
            Token::new_pos(POS::Noun),
            Token::new_str("!"),
        ];
        assert_eq!(got.body, exp);
        assert_eq!(got.title, Some("the bowman".to_string()));
    }
    #[test]
    fn colon_in_literal() {
        let got = Template::parse(
            "it's simple: it's not a template at all")
            .unwrap();
        let exp = vec![
            Token::new_str("it's simple: it's not a template at all"),
        ];
        assert_eq!(got.body, exp);
        assert_eq!(got.title, None);
    }
    #[test]
    fn links_and_brackets() {
        let source = "https://stuffdotcom.com/stuff%20cool?thing=neat also <other stuff>";
        let got = sanitize_source(source);
        let exp = " also ";
        assert_eq!(got, exp);
    }
}

