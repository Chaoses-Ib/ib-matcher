/*!
This module provides routines for searching strings for matches of a [regular
expression] (aka "regex"). The regex syntax supported by this crate is similar
to other regex engines, but it lacks several features that are not known how to
implement efficiently. This includes, but is not limited to, look-around and
backreferences. In exchange, all regex searches in this crate have worst case
`O(m * n)` time complexity, where `m` is proportional to the size of the regex
and `n` is proportional to the size of the string being searched.

[regular expression]: https://en.wikipedia.org/wiki/Regular_expression

If you just want API documentation, then skip to the [`cp::Regex`] or [`lita::Regex`] type. See also [choosing a matcher](crate#choosing-a-matcher).

Most of the API is the same as [`regex-automata`](https://docs.rs/regex-automata/), the regex engine used by [`regex`](https://docs.rs/regex/).

# Syntax
Supported syntax:
- Traditional regex (same as the `regex` crate)

  See [`ib_matcher::syntax::regex`](crate::syntax::regex) for details.

  The following examples all use this syntax.
- glob: See [`ib_matcher::syntax::glob`](crate::syntax::glob).

# Usage
```sh
$ cargo add ib_matcher --features regex
```

```
use ib_matcher::regex::cp::Regex;

fn main() {
    let re = Regex::new(r"Hello (?<name>\w+)!").unwrap();
    let mut caps = re.create_captures();
    let hay = "Hello Murphy!";
    let Ok(()) = re.captures(hay, &mut caps) else {
        println!("no match!");
        return;
    };
    println!("The name is: {}", &hay[caps.get_group_by_name("name").unwrap()]);
}
```

# Examples

This section provides a few examples, in tutorial style, showing how to
search a haystack with a regex. There are more examples throughout the API
documentation.

Before starting though, it's worth defining a few terms:

* A **regex** is a Rust value whose type is `Regex`. We use `re` as a
variable name for a regex.
* A **pattern** is the string that is used to build a regex. We use `pat` as
a variable name for a pattern.
* A **haystack** is the string that is searched by a regex. We use `hay` as a
variable name for a haystack.

Sometimes the words "regex" and "pattern" are used interchangeably.

General use of regular expressions in this crate proceeds by compiling a
**pattern** into a **regex**, and then using that regex to search, split or
replace parts of a **haystack**.

### Validating a particular date format

This examples shows how to confirm whether a haystack, in its entirety, matches
a particular date format:

```rust
use ib_matcher::regex::cp::Regex;

let re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
assert!(re.is_match("2010-03-14"));
```

Notice the use of the `^` and `$` anchors. In this crate, every regex search is
run with an implicit `(?s:.)*?` at the beginning of its pattern, which allows
the regex to match anywhere in a haystack. Anchors, as above, can be used to
ensure that the full haystack matches a pattern.

This crate is also Unicode aware by default, which means that `\d` might match
more than you might expect it to. For example:

```rust
use ib_matcher::regex::cp::Regex;

let re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
assert!(re.is_match("𝟚𝟘𝟙𝟘-𝟘𝟛-𝟙𝟜"));
```

To only match an ASCII decimal digit, all of the following are equivalent:

* `[0-9]`
* `(?-u:\d)`
* `[[:digit:]]`
* `[\d&&\p{ascii}]`

### Finding dates in a haystack

In the previous example, we showed how one might validate that a haystack,
in its entirety, corresponded to a particular date format. But what if we wanted
to extract all things that look like dates in a specific format from a haystack?
To do this, we can use an iterator API to find all matches (notice that we've
removed the anchors and switched to looking for ASCII-only digits):

```rust
use ib_matcher::regex::cp::Regex;

let re = Regex::new(r"[0-9]{4}-[0-9]{2}-[0-9]{2}").unwrap();
let hay = "What do 1865-04-14, 1881-07-02, 1901-09-06 and 1963-11-22 have in common?";
// 'm' is a 'Match', and 'span()' returns the matching part of the haystack.
let dates: Vec<&str> = re.find_iter(hay).map(|m| &hay[m.span()]).collect();
assert_eq!(dates, vec![
    "1865-04-14",
    "1881-07-02",
    "1901-09-06",
    "1963-11-22",
]);
```

### Finding a middle initial

We'll start off with a very simple example: a regex that looks for a specific
name but uses a wildcard to match a middle initial. Our pattern serves as
something like a template that will match a particular name with *any* middle
initial.

```rust
use ib_matcher::regex::cp::Regex;

// We use 'unwrap()' here because it would be a bug in our program if the
// pattern failed to compile to a regex. Panicking in the presence of a bug
// is okay.
let re = Regex::new(r"Homer (.)\. Simpson").unwrap();
let mut caps = re.create_captures();
let hay = "Homer J. Simpson";
let Ok(()) = re.captures(hay, &mut caps) else { return };
assert_eq!("J", &hay[caps.get_group(1).unwrap()]);
```

There are a few things worth noticing here in our first example:

* The `.` is a special pattern meta character that means "match any single
character except for new lines." (More precisely, in this crate, it means
"match any UTF-8 encoding of any Unicode scalar value other than `\n`.")
* We can match an actual `.` literally by escaping it, i.e., `\.`.
* We use Rust's [raw strings] to avoid needing to deal with escape sequences in
both the regex pattern syntax and in Rust's string literal syntax. If we didn't
use raw strings here, we would have had to use `\\.` to match a literal `.`
character. That is, `r"\."` and `"\\."` are equivalent patterns.
* We put our wildcard `.` instruction in parentheses. These parentheses have a
special meaning that says, "make whatever part of the haystack matches within
these parentheses available as a capturing group." After finding a match, we
access this capture group with `caps.get_group(1)`.

[raw strings]: https://doc.rust-lang.org/stable/reference/tokens.html#raw-string-literals

Otherwise, we execute a search using `re.captures(hay)` and return from our
function if no match occurred. We then reference the middle initial by asking
for the part of the haystack that matched the capture group indexed at `1`.
(The capture group at index 0 is implicit and always corresponds to the entire
match. In this case, that's `Homer J. Simpson`.)

### Named capture groups

Continuing from our middle initial example above, we can tweak the pattern
slightly to give a name to the group that matches the middle initial:

```rust
use ib_matcher::regex::cp::Regex;

// Note that (?P<middle>.) is a different way to spell the same thing.
let re = Regex::new(r"Homer (?<middle>.)\. Simpson").unwrap();
let mut caps = re.create_captures();
let hay = "Homer J. Simpson";
let Ok(()) = re.captures(hay, &mut caps) else { return };
assert_eq!("J", &hay[caps.get_group_by_name("middle").unwrap()]);
```

Giving a name to a group can be useful when there are multiple groups in
a pattern. It makes the code referring to those groups a bit easier to
understand.

### Anchored search

This example shows how to use [`Input::anchored`] to run an anchored
search, even when the regex pattern itself isn't anchored. An anchored
search guarantees that if a match is found, then the start offset of the
match corresponds to the offset at which the search was started.

```
use ib_matcher::regex::{cp::Regex, Anchored, Input, Match};

let re = Regex::new(r"\bfoo\b")?;
let input = Input::new("xx foo xx").range(3..).anchored(Anchored::Yes);
// The offsets are in terms of the original haystack.
assert_eq!(Some(Match::must(0, 3..6)), re.find(input));

// Notice that no match occurs here, because \b still takes the
// surrounding context into account, even if it means looking back
// before the start of your search.
let hay = "xxfoo xx";
let input = Input::new(hay).range(2..).anchored(Anchored::Yes);
assert_eq!(None, re.find(input));
// Indeed, you cannot achieve the above by simply slicing the
// haystack itself, since the regex engine can't see the
// surrounding context. This is why 'Input' permits setting
// the bounds of a search!
let input = Input::new(&hay[2..]).anchored(Anchored::Yes);
// WRONG!
assert_eq!(Some(Match::must(0, 0..3)), re.find(input));

# Ok::<(), Box<dyn std::error::Error>>(())
```

### Earliest search

This example shows how to use [`Input::earliest`] to run a search that
might stop before finding the typical leftmost match.

```ignore
use ib_matcher::regex::{cp::Regex, Anchored, Input, Match};

let re = Regex::new(r"[a-z]{3}|b")?;
let input = Input::new("abc").earliest(true);
assert_eq!(Some(Match::must(0, 1..2)), re.find(input));

// Note that "earliest" isn't really a match semantic unto itself.
// Instead, it is merely an instruction to whatever regex engine
// gets used internally to quit as soon as it can. For example,
// this regex uses a different search technique, and winds up
// producing a different (but valid) match!
let re = Regex::new(r"abc|b")?;
let input = Input::new("abc").earliest(true);
assert_eq!(Some(Match::must(0, 0..3)), re.find(input));

# Ok::<(), Box<dyn std::error::Error>>(())
```

### Changing the line terminator

This example shows how to enable multi-line mode by default and change
the line terminator to the NUL byte:

```
use ib_matcher::regex::{cp::Regex, util::{syntax, look::LookMatcher}, Match};

let mut lookm = LookMatcher::new();
lookm.set_line_terminator(b'\x00');
let re = Regex::builder()
    .syntax(syntax::Config::new().multi_line(true))
    .configure(Regex::config().look_matcher(lookm))
    .build(r"^foo$")?;
let hay = "\x00foo\x00";
assert_eq!(Some(Match::must(0, 1..4)), re.find(hay));

# Ok::<(), Box<dyn std::error::Error>>(())
```

### Multi-pattern searches with capture groups

One of the more frustrating limitations of `RegexSet` in the `regex` crate
(at the time of writing) is that it doesn't report match positions. With this
crate, multi-pattern support was intentionally designed in from the beginning,
which means it works in all regex engines and even for capture groups as well.

This example shows how to search for matches of multiple regexes, where each
regex uses the same capture group names to parse different key-value formats.

```
use ib_matcher::regex::{cp::Regex, PatternID};

let re = Regex::builder().build_many(&[
    r#"(?m)^(?<key>[[:word:]]+)=(?<val>[[:word:]]+)$"#,
    r#"(?m)^(?<key>[[:word:]]+)="(?<val>[^"]+)"$"#,
    r#"(?m)^(?<key>[[:word:]]+)='(?<val>[^']+)'$"#,
    r#"(?m)^(?<key>[[:word:]]+):\s*(?<val>[[:word:]]+)$"#,
])?;
let hay = r#"
best_album="Blow Your Face Out"
best_quote='"then as it was, then again it will be"'
best_year=1973
best_simpsons_episode: HOMR
"#;
let mut kvs = vec![];
for caps in re.captures_iter(hay) {
    // N.B. One could use capture indices '1' and '2' here
    // as well. Capture indices are local to each pattern.
    // (Just like names are.)
    let key = &hay[caps.get_group_by_name("key").unwrap()];
    let val = &hay[caps.get_group_by_name("val").unwrap()];
    kvs.push((key, val));
}
assert_eq!(kvs, vec![
    ("best_album", "Blow Your Face Out"),
    ("best_quote", "\"then as it was, then again it will be\""),
    ("best_year", "1973"),
    ("best_simpsons_episode", "HOMR"),
]);

# Ok::<(), Box<dyn std::error::Error>>(())
```
*/
#[cfg(feature = "regex-cp")]
pub mod cp;
#[cfg(feature = "regex-lita")]
pub mod lita;
#[cfg(feature = "regex-nfa")]
pub mod nfa;
#[cfg(feature = "regex-lita")]
pub use regex_automata::dfa;
pub mod util;

pub use regex_automata::{
    Anchored, HalfMatch, Input, Match, MatchError, MatchErrorKind, MatchKind,
    PatternID, Span,
};
#[cfg(feature = "alloc")]
pub use regex_automata::{PatternSet, PatternSetInsertError, PatternSetIter};
