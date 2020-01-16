how do i make a template anyway
===============================

any text not in braces is consider raw text, it is unedited. any html in the
toot is stripped, but the contained text remains

so you can put words (a part-of-speech) into `[]` brackets to indicate a word to
be replaced, like an underline in madlibs. here are the words you can use at
this time:

| word        | example |
| ----------- | ------- |
| adjective   | slimy   |
| comparative | warmer  |
| superlative | coolest |
| noun        | volcano |
| nouns       | sheep   |
| proper      | John    |
| propers     | Alices  |
| pronoun     | she     |
| possessive  | his     |
| adverb      | sweetly |
| uh          | uh      |
| verb        | eat     |
| verbs       | eats    |
| verbed      | ate     |
| participle  | eaten   |
| verbing     | eating  |
| question    | what    |

you may NOT nest `[]` brackets. using `[]` brackets with an incorrect word will
have it filled with a random word

if there is a colon between the beginning and the first template word, all
text up to it will be considered the title. **this also enters manual mode**,
which means that a post will be made asking for fedizens to contribute their own
word suggestions

how do i respond to a manual mode madlibs
=========================================

each response is separated by a newline  
or a comma,

each response starts with a part-of-speech tag exactly like above, then is
followed by a colon(:), then any text besides more colons. *all whitespace
is ignored*, including after the colon

if a line does not contain a colon, it is ignored, and considered a comment

