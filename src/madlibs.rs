// Deals with the madlibs logic: templates, POS, filling in, etc

extern crate senna;
extern crate regex;
extern crate rand;
extern crate serde;
extern crate stopwords;

use self::senna::pos::POS;
use self::senna::senna::*;
use self::stopwords::Stopwords;

use self::regex::Regex;
use self::rand::Rng;
use std::collections::HashSet;

#[derive(Deserialize, Serialize)]
#[serde(remote = "POS")]
#[allow(non_camel_case_types)]
// Unfortunately we have to redefine this entire enum or it errors with an
// imparsible "non-exhaustive patters" error. Fortunately this probably doesn't
// change too much so it's not THAT bad
enum POSSerde {
    NNP, COM, CD, NNS, JJ, MD, VB, DT, NN, IN, PUNCT, VBZ, VBG, CC, VBD, VBN,
    RB, TO, PRP, RBR, WDT, VBP, RP, PRP_POSS, JJS, POS, QUOT_S, WP, QUOT_B, COL,
    JJR, WRB, EX, DOL, NNPS, WP_POSS, LRB, RRB, PDT, RBS, FW, UH, SYM, LS,
    POUND, PADDING, UNAVAILABLE, NOT_SET,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Token {
    pub text: Option<String>,
    is_placeholder: bool,
    #[serde(default, with = "opt_external_struct")]
    pos: Option<POS>,
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_placeholder {
            if self.text == None {
                write!(f, "{}", self.pos.unwrap().to_str())
            } else {
                write!(f, "{}[{}]", self.text.as_ref().unwrap(), self.pos.unwrap().to_str())
            }
        } else {
            write!(f, "{}", self.text.as_ref().unwrap())
        }
    }
}

mod opt_external_struct {
    use super::{POS, POSSerde};
    use madlibs::serde::{Serialize, Serializer, Deserialize, Deserializer};

    pub fn serialize<S>(value: &Option<POS>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Helper<'a>(#[serde(with = "POSSerde")] &'a POS);

        value.as_ref().map(Helper).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<POS>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper(#[serde(with = "POSSerde")] POS);

        let helper = Option::deserialize(deserializer)?;
        Ok(helper.map(|Helper(external)| external))
    }
}

pub type Template = Vec<Token>;

fn sanitize_all(status: &str) -> String {
    // Single line breaks are represented as <br>s, these must be preserved
    let re = Regex::new(r"<br ?/?>").unwrap();
    let status = re.replace_all(&status, "\n");
    // Double newlines are *wrapped* in <p>s, making this a little hacky
    let re = Regex::new(r"</p>").unwrap();
    let status = re.replace_all(&status, "\n\n");
    // Remove *anything else* in TRUE <> charaters, stripping html
    let re = Regex::new(r"<[^<]*>").unwrap();
    let status = re.replace_all(&status, "");
    status.to_string()
}
fn sanitize_source(status: &str) -> String {
    let status = sanitize_all(status);
    // Remove URLs that make https [plural] and // [noun]
    let re = Regex::new(r"http\w+").unwrap();
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

// Though it returns a template, it's not a template because it's all placeholders, it's actually
// just a labelled status
fn label_status(status: &str) -> Template {
    let status = sanitize_source(status);

    let stops: HashSet<_> = stopwords::Spark::stopwords(stopwords::Language::English).unwrap().iter().collect();

    let mut labelled = Template::new();
    // The data directory I just added from rust-senna submodule because I'm lazy
    let mut senna = Senna::new("rust-senna/senna/".to_string());
    let options = SennaParseOptions {
        psg: false,
        pos: true,
    };
    let sen = senna.parse(&status, options);
    for word in sen.get_words() {
        if !stops.contains(&word.get_string().clone()) {
            let token = Token {
                text: Some(word.get_string().to_string()),
                is_placeholder: true,
                pos: Some(word.get_pos()),
            };
            labelled.push(token);
        }
    }
    labelled
}

fn collect(template: &Template) -> String {
    template.iter().fold(String::new(), |s, token| {
        format!("{}{}", s, token.text.as_ref().unwrap())
    })
}

fn check_done(template: &Template) -> bool {
    // If every single one is not a placeholder, we're done!
    template.iter().all(|token| !token.is_placeholder)
}

// Modifies template in-line
// Returns either Some(fully reduced madlibs string) or None
// Only fills in one word, exits immediately
// (i.e. it's made for one word per status)
pub fn reduce_template(template: &mut Template, status: &str) -> Option<String> {
    let mut status = label_status(&sanitize_source(&status));
    // Don't just take the first one, because that tends to be boring
    let mut rng = rand::thread_rng();
    rng.shuffle(&mut status);
    for loan_word in status {
        // This looks unsafe, but because we exit as soon as we fuck with len
        // This messiness had to happen because the borrow checker hates us
        let mut done = false;
        for template_word in template.iter_mut() {
            // A placeholder matches a word found
            if template_word.is_placeholder && template_word.pos == loan_word.pos {
                // We have found a match!
                template_word.text = loan_word.text;
                template_word.is_placeholder = false;
                // We have to do it this way so the borrow in the for loop can end
                done = true;
                break;
            }
        }
        // Break twice
        if done {
            break;
        }
    }
    match check_done(template) {
        true => Some(collect(template)),
        false => None,
    }
}

fn str_to_pos(name: &str) -> POS {
    match name.as_ref() {
        "adjective" => POS::JJ,
        "comparative" => POS::JJR,
        "superlative" => POS::JJS,
        "noun" => POS::NN,
        "nouns" => POS::NNS,
        "proper" => POS::NNP,
        "propers" => POS::NNPS,
        "pronoun" => POS::PRP,
        "possessive" => POS::PRP_POSS,
        "adverb" => POS::RB,
        "uh" => POS::UH,
        "verb" => POS::VB,
        "verbs" => POS::VBZ,
        "past" => POS::VBD,
        "participle" => POS::VBN,
        "verbing" => POS::VBG,
        "question" => POS::WP,
        _ => POS::UNAVAILABLE,
    }
}

pub fn to_template(status: &str) -> Template {
    let status = sanitize_template(status);

    const OPEN: char = '[';
    const CLOSE: char = ']';
    let mut template = Template::new();
    let mut in_brace = false;
    let mut chunk = String::new();
    for c in status.chars() {
        match c {
            OPEN => {
                assert!(!in_brace, "no nesting");
                in_brace = true;
                let token = Token {
                    text: Some(chunk),
                    is_placeholder: false,
                    pos: None,
                };
                template.push(token);
                chunk = String::new();
            },
            CLOSE => {
                assert!(in_brace, "could not match ] to [");
                in_brace = false;
                let token = Token {
                    text: None,
                    is_placeholder: true,
                    pos: Some(str_to_pos(&chunk)),
                };
                template.push(token);
                chunk = String::new();
            },
            _ => {
                chunk.push(c);
            }
        };
    }
    if in_brace {
        panic!("could not match [ to its ] before toot ended");
    }
    // Always end with a non-placeholder representing the end, or at least ""
    let token = Token {
        text: Some(chunk),
        is_placeholder: false,
        pos: None,
    };
    template.push(token);

    template
}

