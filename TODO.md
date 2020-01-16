here's what i wanna do

- MANUAL MADLIBS!! i think this is a really important part of the madlibs
  experience and i'd like to enable it

  1. accept manual requests using this syntax: `title(str): template`
  2. store titles in templates (have to make a "db" migration)

      - maybe i should use an actual database instead of fucking bincode lol

  3. send out a toot that says something like ```
let's play madlibs! i need:
\- 2 nouns
\- 1 pronoun
\- 3 present-tense verbs

submit one by replying like this: `noun: hegemony`
cc @submitter```
  4. identify replies to a Collection toot, parse them with their own state
  machine parser
  5. remember who replied so you can tag them in the final result?
  6. automatic will run as so:

      - in order to avoid backlog / ensure we resolve Collection toots we're
      gonna keep track of them: there will be N unresolved Collection toots
      - if we have an open collection:
          - const % chance we: post an update to (the oldest?) current
          Collection toot, essentially boosting it in our TL, asking for more
          submissions
          - otherwise: we post an automatic toot
      - if no open collection:
          - const % chance we: post a new Manual Collection toot with `untitled`
          if title is None
          - otherwise: we post an automatic toot
      - note this means titled / previously manual will often by run as
      automatic, but that's totally fine

now here's we we iteratively do that:
- MPV: 1234
- 5 is QOL, 6 is a huge thing for our auto machine
- copy Template into src/bin/migrate-titles.rs, then edit it and use compiler
driven development to bring us 1+3
    - because migrate should only have to run once, make sure you get
    collection_toots in that schema right off the bat
- then actually write migrate-titles
- then write 4

what we have so far:
1234

