madlibs bot!
============

> madlibs, except fediverse selects the words

tell me about this bot
----------------------

here's what the bot does: you can @mention it with a template. it'll read the
toots from all its followers and fill it in with words from their toots

what does a template look like? it pretty much just looks like this:

    [pronoun] was a [noun], [pronoun] was a [noun], can i be aaaanymore [adjective]~

get the idea? a full doc is on its way. anyway, mention `@madlibs@beeping.town`
with a template and it'll do madlibs for you!

i meant about the code
----------------------

i'm new to rust, i have no idea what i'm doing, it's terrible, don't look at it.
anyway, it polls notifications, parses the template according to a tiny DSL i
made into a vector of part-of-speech tokens. then it looks on its home timeline,
and parses the POS of each toot using
[SENNA](https://github.com/jfschaefer/rust-senna/tree/master/src). then it fills
in one word of the template per toot!

ok but how do i run it
----------------------

    $ git clone https://github.com/CosineP/madlibs && cd madlibs
	$ cargo run

yeah, p. simple. the first time you run it, it'll ask you to do a credentials
thing with your web browser. it'll keep track of its own status on your HDD so
don't worry if you have to restart it

how can i help
--------------

you could make the rust not-garbage, if you wanted. send a PR!

