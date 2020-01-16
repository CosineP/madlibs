#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate simple_logger;
extern crate elefren;
extern crate chrono;
extern crate toml;
// Yes, it is worth it for both. TOML doesn't support Vec<Template>,
// and elefren doesn't support anything but TOML
// TODO: I could technically just serialize the credentials to bincode as well
extern crate bincode;
extern crate rand;
extern crate senna;
extern crate regex;
extern crate serde;
extern crate stopwords;
#[macro_use]
extern crate lazy_static;
extern crate bimap;

pub mod pos;
pub mod template;
pub mod collection;
pub mod bot;

// We're gonna store the full handle[@domain] so we can mention, ID can't
// do that
pub type AccountID = String;

use regex::Regex;

fn sanitize_all(status: &str) -> String {
    // Single line breaks are represented as <br>s, these must be preserved
    let re = Regex::new(r"<br ?/?>").unwrap();
    let status = re.replace_all(&status, "\n");
    // Double newlines are *wrapped* in <p>s, making this a little hacky
    let re = Regex::new(r"</p>").unwrap();
    let status = re.replace_all(&status, "\n\n");
    // Mentions and links include names which is weird
    let re = Regex::new(r"<a.*</a.*>").unwrap();
    let status = re.replace_all(&status, "");
    // Remove *anything else* in TRUE <> charaters, stripping html
    let re = Regex::new(r"<[^<]*>").unwrap();
    let status = re.replace_all(&status, "");
    status.to_string()
}

