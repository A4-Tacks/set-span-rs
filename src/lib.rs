#![doc = include_str!("../README.md")]
#![allow(clippy::needless_doctest_main)]

use std::convert::identity;

use proc_macro::{Delimiter, Group, Span, TokenStream, TokenTree as TT};
use proc_macro_tool::{
    err, rerr, try_pfunc, GetSpan, ParseIter, ParseIterExt,
    SetSpan, TokenStreamExt, TokenTreeExt, WalkExt,
};

fn eoi(iter: impl IntoIterator<Item = TT>) -> Result<(), TokenStream> {
    iter.into_iter().next().map_or(Ok(()), |tt| {
        Err(err("unexpected token, expected end of input", tt))
    })
}

fn fmt(g: Group) -> String {
    if g.is_delimiter(Delimiter::None) {
        format!("Ø{g}Ø")
    } else {
        g.to_string()
    }
}

const NUM_SUFFIXS: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128",
    "u8", "u16", "u32", "u64", "u128",
];
trait StrExt {
    fn remove_number_suffix(&self) -> &Self;
}
impl StrExt for str {
    fn remove_number_suffix(&self) -> &Self {
        for &suffix in NUM_SUFFIXS {
            if let Some(s) = self.strip_suffix(suffix) {
                return s;
            }
        }
        self
    }
}

#[must_use]
fn index_tt<I>(mut tt: TT, iter: &mut ParseIter<I>) -> Result<TT, TokenStream>
where I: Iterator<Item = TT>,
{
    while let Some((mut span, mut param)) = iter
        .next_if(|tt| tt.is_delimiter(Delimiter::Bracket))
        .map(|tt| tt.into_group().unwrap())
        .map(|g| (g.span_close(), g.stream().into_iter()))
    {
        let i = param.next()
            .ok_or_else(|| err!("unexpected token, expected literal", span))?
            .span_as(&mut span)
            .into_literal()
            .map_err(|_| err!("unexpected token, expected literal", span))?
            .to_string()
            .remove_number_suffix()
            .parse()
            .map_err(|e| err!(@("parse number {e}"), span))?;
        let g = tt.into_group()
            .map_err(|t| err!(@("cannot index {t}, e.g [...]"), span))?;
        tt = g.stream().into_iter().nth(i)
            .ok_or_else(|| err!(@("index {i} out of range, of {}", fmt(g)), span))?
    };
    Ok(tt)
}

fn parse_input_span<I>(iter: I) -> Result<TT, TokenStream>
where I: Iterator<Item = TT>,
{
    let mut iter = iter.parse_iter();
    if iter.peek_puncts("#").is_some()
    && iter.peek_i_is(1, |t| t.is_keyword("mixed"))
    {
        return Ok(iter.next().unwrap().set_spaned(Span::mixed_site()));
    }

    let mut tt = iter.next()
        .ok_or_else(|| err!("unexpected comma of input start"))?;

    tt = index_tt(tt, &mut iter)?;

    if let Some(end) = iter.next() {
        rerr!("unexpected token, expected [...] or comma", end)
    }

    Ok(tt)
}

fn extract_expand_body<I>(
    input: &mut ParseIter<I>,
    span: Span,
) -> Result<TokenStream, TokenStream>
where I: Iterator<Item = TT>,
{
    let Some(tt) = input.next() else {
        rerr!("unexpected end of input, expected a brace {...}", span)
    };
    let Some(group) = tt.as_group() else {
        rerr!("unexpected token, expected a brace {...}", tt)
    };

    let out = if group.is_solid_group() {
        group.stream()
    } else {
        extract_expand_body(input, group.span())?
    };

    eoi(input)?;

    Ok(out)
}

fn do_operation(
    input: TokenStream,
    spant: TT,
) -> Result<TokenStream, TokenStream> {
    try_pfunc(input, false, [
        "set_span",
        "set_index_span",
    ], |i, param| {
        Ok(match &*i.to_string() {
            "set_span" => {
                param.stream()
                    .walk(|tt| tt.set_spaned(spant.span()))
            },
            "set_index_span" => {
                let iter = &mut param.stream().parse_iter();
                let spant = index_tt(spant.clone(), iter)?;
                let result = iter.next()
                    .ok_or_else(|| err!("unexpected end of input, expected {...}", param))?
                    .into_group()
                    .map_err(|t| err!("expected {...}", t))?
                    .stream()
                    .walk(|tt| tt.set_spaned(spant.span()));
                eoi(iter)?;
                result
            },
            _ => unreachable!(),
        })
    })
}

fn set_span_impl(input: TokenStream) -> Result<TokenStream, TokenStream> {
    let Some((spani, mut input)) = input.split_puncts(",") else {
        rerr!("unexpected end of input, expected comma");
    };
    let span = parse_input_span(spani.into_iter())?;
    let input = extract_expand_body(&mut input, Span::call_site())?;
    do_operation(input, span)
}

fn set_span_all_impl(input: TokenStream) -> Result<TokenStream, TokenStream> {
    let iter = &mut input.parse_iter();
    let spant = iter.next()
        .ok_or_else(|| err!("unexpected end of input, expected any token"))?;
    let spant = index_tt(spant, iter)?;

    iter.next_puncts(",");

    let result = iter.next()
        .ok_or_else(|| err!("unexpected end of input, expected {...}", Span::call_site()))?
        .into_group()
        .map_err(|t| err!("expected {...}", t))?
        .stream()
        .walk(|tt| tt.set_spaned(spant.span()));
    eoi(iter)?;

    Ok(result)
}

/// Set the span of certain tokens in the code block to the span of the input token
///
/// - grammar := span `,` `{` code\* `}`
/// - span := `#mixed` / any-token index\*
/// - index := `[` num-literal `]`
/// - code := `#set_span` `{` ... `}`<br>
///         / `#set_index_span` `{` index\* `{` ... `}` `}`<br>
///         / any-token
///
/// `{`, `}` is general bracket, contain `()` `[]` `{}`
///
/// - `set_span!((a, b, (c))[0], {...})` set `a` span
/// - `set_span!((a, b, (c))[1], {...})` set `b` span
/// - `set_span!((a, b, (c))[2], {...})` set `(c)` span
/// - `set_span!((a, b, (c))[2][0], {...})` set `c` span
///
/// Similarly, there is also
/// `set_span!((a), {#set_index_span([0]{...})})` set `a` span
///
///
/// # Example
/// ```
/// macro_rules! foo {
///     ($t:tt) => {
///         foo! { ($t) ($t) }
///     };
///     ($t:tt (0)) => {
///         set_span::set_span! {$t[0], {
///             #set_span {
///                 compile_error! {"input by zero"}
///             }
///         }}
///     };
///     ($_:tt ($lit:literal)) => { /*...*/ };
/// }
/// // foo!(0); // test error msg
/// ```
#[proc_macro]
pub fn set_span(input: TokenStream) -> TokenStream {
    set_span_impl(input).unwrap_or_else(identity)
}

/// Like `set_span!(tt[0], {#set_span{...}})`
///
/// # Example
/// ```
/// macro_rules! foo {
///     ($t:tt) => {
///         foo! { ($t) ($t) }
///     };
///     ($t:tt (0)) => {
///         set_span::set_span_all! {$t[0], {
///             compile_error! {"input by zero"}
///         }}
///     };
///     ($_:tt ($lit:literal)) => { /*...*/ };
/// }
/// // foo!(0); // test error msg
/// ```
#[proc_macro]
pub fn set_span_all(input: TokenStream) -> TokenStream {
    set_span_all_impl(input).unwrap_or_else(identity)
}
