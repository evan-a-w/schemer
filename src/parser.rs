use nom::branch::alt;
use nom::bytes::streaming::{is_not, take_while_m_n};
use nom::bytes::complete::{tag, take_while1, take};
use nom::character::complete::{char, one_of, multispace0, multispace1};
use nom::combinator::{map, map_opt, map_res, value, verify, opt, recognize,
                      complete, not};
use nom::error::{FromExternalError, ParseError};
use nom::multi::{fold_many0, many1, many0, fill};
use nom::sequence::{delimited, preceded, terminated, pair, tuple};
use nom::multi::{separated_list0, separated_list1};
use nom::IResult;
use crate::types::*;
use std::collections::{HashMap, LinkedList};
use nom::error::{self, ErrorKind};

const DELIMITERS: &str = r#"();"'`|[]{}"#;
const WHITESPACE: &str = " \t\n\r";

// The entire string parsing is taken from the nom examples.

// parser combinators are constructed from the bottom up:
// first we write parsers for the smallest elements (escaped characters),
// then combine them into larger parsers.

/// Parse a unicode sequence, of the form u{XXXX}, where XXXX is 1 to 6
/// hexadecimal numerals. We will combine this later with parse_escaped_char
/// to parse sequences like \u{00AC}.
fn parse_unicode<'a, E>(input: &'a str) -> IResult<&'a str, char, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    // `take_while_m_n` parses between `m` and `n` bytes (inclusive) that match
    // a predicate. `parse_hex` here parses between 1 and 6 hexadecimal numerals.
    let parse_hex = take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit());

    // `preceded` takes a prefix parser, and if it succeeds, returns the result
    // of the body parser. In this case, it parses u{XXXX}.
    let parse_delimited_hex = preceded(
        char('u'),
        // `delimited` is like `preceded`, but it parses both a prefix and a suffix.
        // It returns the result of the middle parser. In this case, it parses
        // {XXXX}, where XXXX is 1 to 6 hex numerals, and returns XXXX
        delimited(char('{'), parse_hex, char('}')),
    );

    // `map_res` takes the result of a parser and applies a function that returns
    // a Result. In this case we take the hex bytes from parse_hex and attempt to
    // convert them to a u32.
    let parse_u32 = map_res(parse_delimited_hex, move |hex| u32::from_str_radix(hex, 16));

    // map_opt is like map_res, but it takes an Option instead of a Result. If
    // the function returns None, map_opt returns an error. In this case, because
    // not all u32 values are valid unicode code points, we have to fallibly
    // convert to char with from_u32.
    map_opt(parse_u32, |value| std::char::from_u32(value))(input)
}

/// Parse an escaped character: \n, \t, \r, \u{00AC}, etc.
fn parse_escaped_char<'a, E>(input: &'a str) -> IResult<&'a str, char, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    preceded(
        char('\\'),
        // `alt` tries each parser in sequence, returning the result of
        // the first successful match
        alt((
            parse_unicode,
            // The `value` parser returns a fixed value (the first argument) if its
            // parser (the second argument) succeeds. In these cases, it looks for
            // the marker characters (n, r, t, etc) and returns the matching
            // character (\n, \r, \t, etc).
            value('\n', char('n')),
            value('\r', char('r')),
            value('\t', char('t')),
            value('\u{08}', char('b')),
            value('\u{0C}', char('f')),
            value('\\', char('\\')),
            value('/', char('/')),
            value('"', char('"')),
        )),
    )(input)
}

/// Parse a backslash, followed by any amount of whitespace. This is used later
/// to discard any escaped whitespace.
fn parse_escaped_whitespace<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, &'a str, E> {
    preceded(char('\\'), multispace1)(input)
}

/// Parse a non-empty block of text that doesn't include \ or "
fn parse_literal<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    // `is_not` parses a string of 0 or more characters that aren't one of the
    // given characters.
    let not_quote_slash = is_not("\"\\");

    // `verify` runs a parser, then runs a verification function on the output of
    // the parser. The verification function accepts out output only if it
    // returns true. In this case, we want to ensure that the output of is_not
    // is non-empty.
    verify(not_quote_slash, |s: &str| !s.is_empty())(input)
}

/// A string fragment contains a fragment of a string being parsed: either
/// a non-empty Literal (a series of non-escaped characters), a single
/// parsed escaped character, or a block of escaped whitespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StringFragment<'a> {
    Literal(&'a str),
    EscapedChar(char),
    EscapedWS,
}

/// Combine parse_literal, parse_escaped_whitespace, and parse_escaped_char
/// into a StringFragment.
fn parse_fragment<'a, E>(input: &'a str) -> IResult<&'a str, StringFragment<'a>, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    alt((
        // The `map` combinator runs a parser, then applies a function to the output
        // of that parser.
        map(parse_literal, StringFragment::Literal),
        map(parse_escaped_char, StringFragment::EscapedChar),
        value(StringFragment::EscapedWS, parse_escaped_whitespace),
    ))(input)
}

/// Parse a string. Use a loop of parse_fragment and push all of the fragments
/// into an output string.
fn parse_string<'a, E>(input: &'a str) -> IResult<&'a str, String, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    // fold_many0 is the equivalent of iterator::fold. It runs a parser in a loop,
    // and for each output value, calls a folding function on each output value.
    let build_string = fold_many0(
        // Our parser function– parses a single string fragment
        parse_fragment,
        // Our init value, an empty string
        String::new,
        // Our folding function. For each fragment, append the fragment to the
        // string.
        |mut string, fragment| {
            match fragment {
                StringFragment::Literal(s) => string.push_str(s),
                StringFragment::EscapedChar(c) => string.push(c),
                StringFragment::EscapedWS => {}
            }
            string
        },
    );

    // Finally, parse the string. Note that, if `build_string` could accept a raw
    // " character, the closing delimiter " would never match. When using
    // `delimited` with a looping parser (like fold_many0), be sure that the
    // loop won't accidentally match your closing delimiter!
    delimited(char('"'), build_string, char('"'))(input)
}

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and 
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F: 'a, O, E: ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
    where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(
        multispace0,
        inner,
        multispace0
    )
}

fn prec_ws<'a, F: 'a, O, E: ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
    where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    preceded(
        multispace0,
        inner,
    )
}

fn hex(input: &str) -> IResult<&str, isize> {
    map_res(
        preceded(
            alt((tag("#x"), tag("#X"))),
            recognize(
                many0(
                    one_of("0123456789abcdefABCDEF")
                )
            )
        ),
        |out: &str| isize::from_str_radix(out, 16)
    )(input)
}

fn octal(input: &str) -> IResult<&str, isize> {
    map_res(
        preceded(
            alt((tag("#o"), tag("#O"))),
            recognize(
                many0(
                    one_of("01234567")
                )
            )
        ),
        |out: &str| isize::from_str_radix(out, 8)
    )(input)
}

fn binary(input: &str) -> IResult<&str, isize> {
    map_res(
        preceded(
            alt((tag("#b"), tag("#B"))),
            recognize(
                many0(
                    one_of("01")
                )
            )
        ),
        |out: &str| isize::from_str_radix(out, 2)
    )(input)
}

fn decimal(input: &str) -> IResult<&str, isize> {
    map_res(
        recognize(
            pair(
                opt(one_of("+-")),
                recognize(
                    many0(
                        one_of("0123456789")
                    )
                )
            )
        ),
        |out: &str| isize::from_str_radix(out, 10)
    )(input)
}

pub fn int_parser(input: &str) -> IResult<&str, isize> {
    alt((hex, octal, binary, decimal))(input)
}

pub fn float_parser(input: &str) -> IResult<&str, f64> {
    map_res(
        recognize(
            tuple((
                opt(one_of("+-")),
                opt(decimal), char('.'), opt(decimal),
                opt(tuple((one_of("eE"), decimal)))
            ))
        ),
        |out: &str| out.parse::<f64>(),
    )(input)
}

pub fn ponga_float_parser(input: &str) -> IResult<&str, Ponga> {
    let (a, res) = float_parser(input)?;
    Ok((a, Ponga::Number(Number::Float(res))))
}

pub fn ponga_int_parser(input: &str) -> IResult<&str, Ponga> {
    let (a, res) = int_parser(input)?;
    Ok((a, Ponga::Number(Number::Int(res))))
}

pub fn num_parser(input: &str) -> IResult<&str, Ponga> {
    alt((
        complete(ponga_float_parser),
        complete(ponga_int_parser),
    ))(input)
}

pub fn array_parser(input: &str) -> IResult<&str, Ponga> {
    map(
        delimited(
            tag("#("),
            separated_list0(
                multispace1,
                ponga_parser,
            ),
            preceded(multispace0, tag(")"))
        ),
        |out: Vec<Ponga>| -> Ponga { Ponga::Array(out) }
    )(input)
}

pub fn list_parser(input: &str) -> IResult<&str, Ponga> {
    map(
        delimited(
            tag("'("),
            separated_list0(
                multispace1,
                ponga_parser,
            ),
            preceded(multispace0, tag(")"))
        ),
        |out: Vec<Ponga>| -> Ponga {
            Ponga::List(out.into_iter().collect())
        }
    )(input)
}

pub fn sexpr_parser(input: &str) -> IResult<&str, Ponga> {
    map(
        delimited(
            tag("("),
            separated_list0(
                multispace1,
                ponga_parser,
            ),
            preceded(multispace0, tag(")"))
        ),
        |out: Vec<Ponga>| -> Ponga {
            Ponga::Sexpr(out)
        }
    )(input)
}

pub fn non_delimiter(input: &str) -> IResult<&str, &str> {
    take_while1(
        |c: char| {
            !(DELIMITERS.contains(&[c]) || WHITESPACE.contains(&[c]))
        }
    )(input)
}

pub fn string_parser(input: &str) -> IResult<&str, Ponga> {
    map(
        parse_string,
        |s: String| -> Ponga { Ponga::String(s) }
    )(input)
}

pub fn identifier_parser(input: &str) -> IResult<&str, Ponga> {
    map(
        recognize(
            tuple((
                not(one_of("#,'")),
                non_delimiter,
            ))
        ),
        move |out: &str| -> Ponga {
            Ponga::Identifier(out.to_string())
        }
    )(input)
}

pub fn symbol_parser(input: &str) -> IResult<&str, Ponga> {
    map_res(
        preceded(
            char('\''),
            identifier_parser
        ),
        move |out: Ponga| -> Result<Ponga, ()> {
            let out = match out {
                Ponga::Identifier(s) => s,
                _ => return Err(()),
            };
            Ok(Ponga::Symbol(out))
        }
    )(input)
}

pub fn true_parser(input: &str) -> IResult<&str, Ponga> {
    map(
        tag("#t"),
        |_| Ponga::True
    )(input)
}

pub fn false_parser(input: &str) -> IResult<&str, Ponga> {
    map(
        tag("#f"),
        |_| Ponga::False
    )(input)
}

pub fn bool_parser(input: &str) -> IResult<&str, Ponga> {
    alt((
        false_parser,
        true_parser
    ))(input)
}

pub fn char_parser(input: &str) -> IResult<&str, Ponga> {
    map(
        preceded(
            tag("#\\"), take(1usize)
        ),
        |x: &str| Ponga::Char(x.chars().nth(0).unwrap()),
    )(input)
}

pub fn ponga_parser(input: &str) -> IResult<&str, Ponga> {
    preceded(
        multispace0,
        alt((
            string_parser,
            char_parser,
            num_parser,
            array_parser,
            list_parser,
            sexpr_parser,
            bool_parser,
            symbol_parser,
            identifier_parser,
        )),
    )(input)
}

pub fn pongascript_parser(input: &str) -> IResult<&str, Vec<Ponga>> {
    many1(
        ponga_parser
    )(input)
}
