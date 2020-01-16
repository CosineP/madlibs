extern crate madlibs;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate senna;

use madlibs::template;
use madlibs::bot;
use std::fs::File;
use std::collections::HashMap;
use senna::pos::POS as SPOS;
use bot::BotStatus;
use template::{Template};

/// well it looks like a mess but on the bright side it's all code we removed
/// from needing to maintain

#[derive(Deserialize)]
#[serde(remote = "SPOS")]
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

mod opt_external_struct {
    use super::{SPOS, POSSerde};
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SPOS>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper(#[serde(with = "POSSerde")] SPOS);

        let helper = Option::deserialize(deserializer)?;
        Ok(helper.map(|Helper(external)| external))
    }
}

#[derive(Deserialize)]
pub struct OldToken {
    pub text: Option<String>,
    is_placeholder: bool,
    #[serde(default, with = "opt_external_struct")]
    pos: Option<SPOS>,
}

pub type OldTemplate = Vec<OldToken>;

#[derive(Deserialize)]
pub struct OldBotStatus {
    pub last_noti_date: chrono::DateTime<chrono::Utc>,
    pub known_templates: Vec<OldTemplate>,
}

fn read_old() -> OldBotStatus {
    match File::open("status.bincode") {
        Ok(file) => {
            bincode::deserialize_from(file).expect("old status struct didn't match given db")
        },
        Err(_) => panic!("couldn't open old db file"),
    }
}

fn write(bot_status: BotStatus) {
    match File::create("status.bincode") {
        Ok(file) => {
            bincode::serialize_into(file, &bot_status)
                .expect("couldn't serialize to file (fs error, not schema)")
        },
        Err(_) => panic!("couldn't create/open status file")
    };
}

fn main() {
    use template::Token;
    use madlibs::pos;
    let old = read_old();
    let mut new_templates = Vec::new();
    for plate in old.known_templates {
        let mut new_body = Vec::new();
        for token in plate {
            new_body.push(Token {
                text: token.text,
                is_placeholder: token.is_placeholder,
                pos: token.pos.map(pos::senna_to_pos).map(Option::unwrap),
            })
        }
        new_templates.push(Template {
            title: None,
            body: new_body,
        })
    }
    let new = BotStatus {
        known_templates: new_templates,
        last_noti_date: old.last_noti_date,
        collection_toots: HashMap::new(),
    };
    write(new);
}

