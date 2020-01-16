use std::collections::HashSet;
use AccountID;
use sanitize_all;
use pos::*;
use template::Template;

pub type Response = (POS, String);

#[derive(Deserialize, Serialize)]
pub struct CollectionStatus {
    // template is not partially resolved, because it's re-used...
    template_id: usize,
    participants: HashSet<AccountID>,
    // ...instead suggestions are stored like this(?)
    resolved: Vec<Response>,
}
impl CollectionStatus {
    pub fn new(template_id: usize, acct: AccountID) -> Self {
        let mut participants = HashSet::new();
        participants.insert(acct);
        Self {
            template_id,
            participants,
            resolved: Vec::new(),
        }
    }
    pub fn add_responses(&mut self, mut resps: Vec<Response>) {
        self.resolved.append(&mut resps);
    }
    pub fn add_participant(&mut self, participant: AccountID) {
        self.participants.insert(participant);
    }
    pub fn get_participant_ats(&self) -> String {
        let mut text = String::from("\ncc");
        for participant in &self.participants {
            text.push_str(" @");
            text.push_str(participant);
        }
        text
    }
    // returns None if not enough data and Some(resolved) if there is
    pub fn check_done(&self, templates: &Vec<Template>) -> Option<String> {
        let mut template_clone = templates[self.template_id].clone();
        // TODO: select randomly
        for resp in &self.resolved {
            template_clone.insert_placeholder(resp.0, resp.1.clone());
        }
        template_clone.check_done()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    DeclaredTwice,
    UnknownPOS(String),
    ExpectedWord,
}
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use self::ParseError::*;
        match self {
            ExpectedWord => write!(f, "gave a pos: but then no word before newline/comma"),
            DeclaredTwice => write!(f, "didn't expect two declarations like `noun: verb: thing`"),
            UnknownPOS(given) => write!(f, "unknown part of speech {}", given),
        }
    }
}
impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

/// actually parses one toot that may contain many responses
pub fn parse_response(resp: &str) -> Result<Vec<Response>, ParseError> {
    let resp = sanitize_all(resp);

    const DECLARE: char = ':';
    const SEP: char = '\n';
    const SEP2: char = ',';
    let mut responses = Vec::new();
    let mut chunk = String::new();
    let mut pos = None;
    for c in resp.chars() {
        match c {
            DECLARE => {
                if pos.is_some() {
                    return Err(ParseError::DeclaredTwice);
                }
                pos = match str_to_pos(&chunk) {
                    Some(p) => Some(p),
                    None => return Err(ParseError::UnknownPOS(chunk)),
                };
                chunk = String::new();
            },
            SEP | SEP2 => {
                if let Some(pos) = pos {
                    if chunk != "" {
                        responses.push((pos, chunk));
                    } else {
                        return Err(ParseError::ExpectedWord);
                    }
                }
                // if there wasn't anything, then this is noise / comment
                pos = None;
                chunk = String::new();
            },
            // ignore these whitespaces
            ' ' | '\t' => (),
            _ => {
                chunk.push(c);
            }
        };
    }
    // deal with the last bit (DRY?)
    if let Some(pos) = pos {
        if chunk != "" {
            responses.push((pos, chunk));
        } else {
            return Err(ParseError::ExpectedWord);
        }
    }
    Ok(responses)
}

#[cfg(test)]
mod test {
    use super::{parse_response, POS, CollectionStatus};
    #[test]
    fn one_decl() {
        let got = parse_response("nouns: cars");
        let exp = vec![(POS::Nouns, "cars".to_string())];
        assert_eq!(got, Ok(exp));
    }
    #[test]
    fn two_decl() {
        let got = parse_response("verbs: eats, uh: grr");
        let exp = vec![
            (POS::Verbs, "eats".to_string()),
            (POS::Uh, "grr".to_string()),
        ];
        assert_eq!(got, Ok(exp));
    }
    #[test]
    fn comment_lines() {
        let got = parse_response("<a href=aoesutnhaoesn>@madlibs</a> verbs: eats, and what else, uhhhh, okay so, uh: grr");
        let exp = vec![
            (POS::Verbs, "eats".to_string()),
            (POS::Uh, "grr".to_string()),
        ];
        assert_eq!(got, Ok(exp));
    }
    #[test]
    fn resolve() {
        use template::Template;
        let req = Template::parse("titled: i need a [noun] another [noun] and a [verb]").unwrap();
        let templates = vec![req];
        let resps = parse_response("noun: thing, noun: table, verb: bore").unwrap();
        let mut cs = CollectionStatus::new(0, "cosine@anticapitalist.party".to_string());
        cs.add_responses(resps);
        let exp = "titled:\n i need a thing another table and a bore".to_string();
        assert_eq!(cs.check_done(&templates), Some(exp));
    }
}

