extern crate senna;
extern crate regex;

use madlibs::senna::pos::POS;
use madlibs::senna::senna::*;

use madlibs::regex::Regex;

pub struct Token {
    pub text: Option<String>,
    is_placeholder: bool,
    pos: Option<POS>,
}

pub type Template = Vec<Token>;

fn strip_html(status: String) -> String {
    let re = Regex::new(r"<[^<]*>").unwrap();
    re.replace_all(&status, " ").to_string()
}

// Though it returns a template, it's not a template because it's all placeholders, it's actually
// just a labelled status
fn label_status(status: String) -> Template {
    let status = strip_html(status);

    let mut labelled = Template::new();
    // The data directory I just added from rust-senna submodule because I'm lazy
    let mut senna = Senna::new("rust-senna/senna/".to_string());
    let options = SennaParseOptions {
        psg: false,
        pos: true,
    };
    let sen = senna.parse(&status, options);
    for word in sen.get_words() {
        let token = Token {
            text: Some(word.get_string().to_string()),
            is_placeholder: true,
            pos: Some(word.get_pos()),
        };
        labelled.push(token);
    }

    labelled
}

fn collect(template: &Template) -> String {
    template.iter().fold(String::new(), |mut s, token| {
        s.push_str(&token.text.clone().unwrap()); s
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
pub fn reduce_template(template: &mut Template, status: String) -> Option<String> {
    let status = label_status(strip_html(status));
    for loan_word in status {
        // This looks unsafe, but because we exit as soon as we fuck with len
        // This messiness had to happen because the borrow checker hates us
        let mut done = false;
        for template_word in template.iter_mut() {
            // A placeholder matches a word found
            if template_word.is_placeholder && template_word.pos == loan_word.pos {
                // We have found a match!
                template_word.text = loan_word.text.clone();
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
        true => Some(collect(&template)),
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

pub fn to_template(status: String) -> Template {
    let status = strip_html(status);

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

