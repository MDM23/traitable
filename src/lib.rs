use std::{
    collections::HashMap,
    iter::{once, Peekable},
};

use proc_macro2::{
    token_stream::IntoIter, Delimiter, Group, Ident, Literal, Punct, Span, TokenStream, TokenTree,
};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::FatArrow,
};
use traitable_core::{cargo::entry_file_from_env, parse, Implementer};

#[proc_macro]
pub fn generate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as GenerateInput);

    let entry = entry_file_from_env().unwrap();
    let result = parse(entry).unwrap();
    let implementers = result.lookup(&input.trait_name.to_string());

    Context::from_iter(implementers)
        .translate(input.body)
        .into()
}

struct GenerateInput {
    trait_name: Ident,
    body: TokenStream,
}

impl Parse for GenerateInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let sig;
        parenthesized!(sig in input);

        input.parse::<FatArrow>()?;

        let body;
        braced!(body in input);

        Ok(GenerateInput {
            trait_name: sig.parse()?,
            body: body.parse()?,
        })
    }
}

#[derive(Default)]
struct Context {
    /// Variables that can be used in the current context. For example: $count
    /// or $name.
    vars: HashMap<String, TokenStream>,

    /// A group of child Context instances that can be used for repetition in
    /// the current context. For example: $( $path $name, )*
    repeatable: Option<Vec<Context>>,
}

impl FromIterator<Implementer> for Context {
    fn from_iter<T: IntoIterator<Item = Implementer>>(iter: T) -> Self {
        let implementers: Vec<_> = iter
            .into_iter()
            .enumerate()
            .map(|(index, imp)| {
                let mut ty_full: Vec<_> = imp.path.split("::").collect();
                ty_full.push(&imp.name);

                let ty_full: Vec<_> = ty_full
                    .iter()
                    .flat_map(|seg| {
                        vec![
                            TokenTree::Ident(Ident::new(seg, Span::call_site())),
                            TokenTree::Punct(Punct::new(':', proc_macro2::Spacing::Joint)),
                            TokenTree::Punct(Punct::new(':', proc_macro2::Spacing::Alone)),
                        ]
                    })
                    .take(ty_full.len() * 3 - 2)
                    .collect();

                Context {
                    vars: HashMap::from_iter([
                        (
                            "index".into(),
                            TokenStream::from(TokenTree::Literal(Literal::usize_unsuffixed(index))),
                        ),
                        (
                            "ty".into(),
                            TokenStream::from(TokenTree::Ident(Ident::new(
                                &imp.name,
                                Span::call_site(),
                            ))),
                        ),
                        ("ty_full".into(), TokenStream::from_iter(ty_full)),
                    ]),
                    repeatable: None,
                }
            })
            .collect();

        Context {
            vars: HashMap::from_iter([(
                "count".into(),
                TokenStream::from(TokenTree::Literal(Literal::usize_unsuffixed(
                    implementers.len(),
                ))),
            )]),
            repeatable: Some(implementers),
        }
    }
}

impl Context {
    pub fn translate(&self, input: TokenStream) -> TokenStream {
        let mut tokens = input.into_iter().peekable();
        let mut output = TokenStream::new();

        loop {
            let Some(tt) = tokens.next() else {
                break;
            };

            match tt {
                TokenTree::Punct(p) if p.as_char() == '$' => {
                    output.extend(self.translate_expression(&mut tokens));
                }
                TokenTree::Group(g) => {
                    output.extend(once(TokenTree::Group(Group::new(
                        g.delimiter(),
                        self.translate(g.stream()),
                    ))));
                }
                other => {
                    output.extend(once(other));
                }
            }
        }

        output
    }

    fn translate_expression(&self, input: &mut Peekable<IntoIter>) -> TokenStream {
        let Some(tt) = input.next() else {
            panic!();
        };

        match tt {
            TokenTree::Ident(id) => self.vars.get(&id.to_string()).cloned().unwrap(),
            TokenTree::Group(gr) => match gr.delimiter() {
                Delimiter::Bracket => {
                    TokenStream::from(TokenTree::Ident(self.make_ident(gr.stream())))
                }
                Delimiter::Parenthesis => {
                    let Some(ref rep) = self.repeatable else {
                        panic!()
                    };

                    let mut out = TokenStream::new();

                    for ctx in rep {
                        out.extend(ctx.translate(gr.stream()));
                    }

                    match input.peek() {
                        Some(TokenTree::Punct(p)) => match p.as_char() {
                            '+' => {
                                input.next();

                                if rep.is_empty() {
                                    panic!("Empty repetition group");
                                }
                            }
                            '*' => {
                                input.next();
                            }
                            _ => (),
                        },
                        _ => (),
                    }

                    out
                }
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    fn make_ident(&self, input: TokenStream) -> Ident {
        let mut name = String::new();

        for tt in self.translate(input) {
            match tt {
                TokenTree::Group(_) => panic!(),
                TokenTree::Ident(id) => {
                    name.push_str(&id.to_string());
                }
                TokenTree::Punct(p) => {
                    name.push(p.as_char());
                }
                TokenTree::Literal(lit) => {
                    // TODO: Remove all forbidden characters
                    name.push_str(&lit.to_string().trim_matches('"'));
                }
            }
        }

        Ident::new(&name, Span::call_site())
    }
}

#[test]
fn test_simple_translation() {
    let input: TokenStream = "pub struct Foobar ( usize, String )".parse().unwrap();

    // dbg!(Context::default().translate(input));

    // todo!()
}

#[test]
fn test_simple_vars() {
    let ctx = Context {
        vars: HashMap::from_iter([(
            String::from("foo"),
            TokenStream::from(TokenTree::Ident(Ident::new("Foobar", Span::call_site()))),
        )]),
        repeatable: None,
    };

    let input: TokenStream = "pub struct $foo;".parse().unwrap();

    // dbg!(ctx.translate(input));

    // todo!()
}

#[test]
fn test_identifiers() {
    let ctx = Context {
        vars: HashMap::from_iter([
            (
                String::from("name"),
                TokenStream::from(TokenTree::Ident(Ident::new("DoLogin", Span::call_site()))),
            ),
            (
                String::from("path"),
                TokenStream::from_iter([
                    TokenTree::Ident(Ident::new("foo", Span::call_site())),
                    TokenTree::Punct(Punct::new(':', proc_macro2::Spacing::Joint)),
                    TokenTree::Punct(Punct::new(':', proc_macro2::Spacing::Alone)),
                    TokenTree::Ident(Ident::new("bar", Span::call_site())),
                ]),
            ),
        ]),
        repeatable: None,
    };

    let input: TokenStream = "enum Bar { $[ $name _Req \"123\" ]($path :: $name) }"
        .parse()
        .unwrap();

    // dbg!(ctx.translate(input).to_string());

    // todo!()
}

#[test]
fn test_repetition() {
    let ctx = Context {
        vars: HashMap::new(),
        repeatable: Some(vec![
            Context {
                vars: HashMap::from_iter([(
                    String::from("name"),
                    TokenStream::from(TokenTree::Ident(Ident::new("DoLogin", Span::call_site()))),
                )]),
                repeatable: None,
            },
            Context {
                vars: HashMap::from_iter([(
                    String::from("name"),
                    TokenStream::from(TokenTree::Ident(Ident::new("DoLogout", Span::call_site()))),
                )]),
                repeatable: None,
            },
        ]),
    };

    dbg!("foo::bar::Goo".parse::<TokenStream>());

    let input: TokenStream = "enum Bar { $($[ $name _Req \"123\" ](usize),)+ }"
        .parse()
        .unwrap();

    // dbg!(ctx.translate(input).to_string());

    // todo!()
}
