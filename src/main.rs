// Deals with the botty aspects: polling, sending, etc

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate simple_logger;

mod madlibs;
mod bot;

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    bot::run();
}

