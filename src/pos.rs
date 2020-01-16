use senna::pos::POS as SPOS;
use bimap::BiMap;

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
// Unfortunately we have to redefine this entire enum or it errors with an
// imparsible "non-exhaustive patters" error. Fortunately this probably doesn't
// change too much so it's not THAT bad
pub enum POS {
    Adjective,
    Comparative,
    Superlative,
    Noun,
    Nouns,
    Proper,
    Propers,
    Pronoun,
    Possessive,
    Adverb,
    Uh,
    Verb,
    Verbs,
    Verbed,
    Participle,
    Verbing,
    Question,
}

lazy_static! {
    static ref STR_TO_POS: BiMap<&'static str, POS> = {
        use self::POS::*;
        let mut m = BiMap::new();
        m.insert("adjective", Adjective);
        m.insert("comparative", Comparative);
        m.insert("superlative", Superlative);
        m.insert("noun", Noun);
        m.insert("nouns", Nouns);
        m.insert("proper", Proper);
        m.insert("propers", Propers);
        m.insert("pronoun", Pronoun);
        m.insert("possessive", Possessive);
        m.insert("adverb", Adverb);
        m.insert("uh", Uh);
        m.insert("verb", Verb);
        m.insert("verbs", Verbs);
        m.insert("verbed", Verbed);
        m.insert("participle", Participle);
        m.insert("verbing", Verbing);
        m.insert("question", Question);
        m
    };
}
pub fn str_to_pos(name: &str) -> Option<POS> {
    STR_TO_POS.get_by_left(&name).cloned()
}
pub fn pos_to_str(pos: &POS) -> &'static str {
    STR_TO_POS.get_by_right(pos).unwrap()
}

// can't use a bimap because SPOS doesn't implement hash, hence this WHOLE
// FUCKERY
pub fn senna_to_pos(pos: SPOS) -> Option<POS> {
    use self::POS::*;
    use self::SPOS::*;
    Some(match pos {
        JJ => Adjective,
        JJR => Comparative,
        JJS => Superlative,
        NN => Noun,
        NNS => Nouns,
        NNP => Proper,
        NNPS => Propers,
        PRP => Pronoun,
        PRP_POSS => Possessive,
        RB => Adverb,
        UH => Uh,
        VB => Verb,
        VBZ => Verbs,
        VBD => Verbed,
        VBN => Participle,
        VBG => Verbing,
        WP => Question,
        _ => return None,
    })
}

