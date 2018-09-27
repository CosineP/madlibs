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

// Modifies template in-line
// Returns whether the template has been fully reduced
pub fn reduce_template(template: &mut Template, status: String) -> bool {
    let status = label_status(strip_html(status));
    for loan_word in status {
        // This looks unsafe, but because we exit as soon as we fuck with len
        // This messiness had to happen because the borrow checker hates us
        for (t_i, template_word) in template.iter_mut().enumerate() {
            // A placeholder matches a word found
            if template_word.is_placeholder && template_word.pos == loan_word.pos {
                // We have found a match!
                template_word.text = loan_word.text.clone();
                template_word.is_placeholder = false;
                // Merge the adjacent non-placeholder parts into one chunk
                // The one ahead has to go first because indices will change
                // if t_i+1 < template.len() && !template[t_i+1].is_placeholder {
                //     match &template[t_i+1].text.clone() {
                //         Some(append_text) => {
                //             template[t_i].text.as_mut().unwrap()
                //                 .push_str(&append_text.clone());
                //         },
                //         None => panic!("non-placeholder section had no next")
                //     };
                //     template.remove(t_i+1);
                // }
                // if t_i > 0 && !template[t_i-1].is_placeholder {
                //     match template[t_i].text.clone() {
                //         Some(append_text) => {
                //             template[t_i-1].text.as_mut().unwrap()
                //                 .push_str(&append_text.clone());
                //         },
                //         None => panic!("non-placeholder section had no next")
                //     };
                //     template.remove(t_i);
                // }
                // return template.len() == 1;
            }
        }
    }
    // Collapse adjacent non-placoholder chunks just created
    let template: Template = template.iter().fold(Vec::new(), |mut plate, &token| {
        let last_placeholder = match plate.last() {
            Some(last) => last.is_placeholder,
            None => false,
        };
        if !last_placeholder && !token.is_placeholder {
            match token.text.clone() {
                Some(token_text) => {
                    plate.last_mut().map(|mut last| {
                        last.text = last.text.clone().map(|last_text| {
                            // last_text.push_str(&token_text)
                            format!("{}{}", last_text, token_text)
                        });
                        last
                    });
                },
                None => panic!("token had no text despite not being placeholder")
            }
        } else {
            plate.push(token);
        }
        plate
    });
    println!("{}", template.len());
    return template.len() == 1;
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

    template
}

