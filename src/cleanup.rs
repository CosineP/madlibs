#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate simple_logger;

extern crate bincode;

mod madlibs;
#[allow(dead_code)]
mod bot;

use std::fs::File;

fn main() {
    let mut status : bot::BotStatus = match File::open("status.bincode") {
        Ok(file) => {
            bincode::deserialize_from(file).unwrap()
        },
        Err(_) => panic!("status.bincode does not exist, no migration necessary or possible")
    };
    // Remove templates that were replies to toots / mentions with NO placeholders, which means
    // they weren't meant to be madlibbed
    status.known_templates.retain(|template| {
        template.len() > 1
    });
    match File::create("status.bincode") {
        Ok(file) => {
            bincode::serialize_into(file, &status)
                .expect("couldn't serialize to file")
        },
        Err(_) => panic!("couldn't create/open status file")
    };
}

