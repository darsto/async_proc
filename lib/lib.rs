/* SPDX-License-Identifier: MIT
 * Copyright(c) 2024 Darek Stojaczyk
 */

extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::TokenTree as TokenTree2;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::token::Underscore;
use syn::{parse_macro_input, Ident, Token};

/// Whole input inside select!
struct SelectInput {
    items: Vec<SelectItem>,
}

/// One arm of select!.
/// The important bit is that we're not parsing any input code, just
/// aggregating the tokens until some character is found. This makes
/// the code blocks like this get "parsed" successfully and provide
/// normal code completions even if the input code is broken:
/// ```ignore
/// p = async_fn() => {
///     p.<caret here>
/// }
/// ```
/// If we decided to parse into syn's Expr or Block, the parsing
/// would fail, then we wouldn't get any code completions at all.
struct SelectItem {
    var_name: VarName,
    expr: Option<TokenStream2>,
    body: TokenStream2,
}

/// Either
///   `let varname = ... {`
/// or
///   `default {`
///   `complete {`
enum VarName {
    Ident(Ident),
    Special,
}

impl Parse for SelectInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut items = Vec::new();

        while !input.is_empty() {
            // 'varname =' or '_ =' or 'default' or 'complete'
            let var_name: VarName = if input.peek(Ident) {
                let ident: Ident = input.parse()?;
                match ident.to_string().as_str() {
                    "default" => VarName::Special,
                    "complete" => VarName::Special,
                    _ => VarName::Ident(ident),
                }
            } else {
                let underscore: Underscore = input.parse()?;
                VarName::Ident(Ident::new("_", underscore.span()))
            };

            let expr = match &var_name {
                VarName::Ident(..) => {
                    input.parse::<Token![=]>()?;
                    let mut expr_tokens = TokenStream2::new();

                    // collect tokens until `=>`
                    while !input.peek(Token![=>]) && !input.is_empty() {
                        expr_tokens.extend(Some(input.parse::<TokenTree2>()?));
                    }

                    input.parse::<Token![=>]>()?;
                    Some(expr_tokens)
                }
                VarName::Special => None,
            };

            // collect tokens inside {} braces
            let body: TokenStream2 = {
                let content;
                syn::braced!(content in input);
                content.parse::<TokenStream2>()?
            };

            items.push(SelectItem {
                var_name,
                expr,
                body,
            });

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(SelectInput { items })
    }
}

#[allow(dead_code)]
fn dummy_select_for_ide(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as SelectInput);

    let output = input.items.into_iter().map(|item| {
        let var_name = item.var_name;
        let expr = item.expr;
        let body = item.body;

        match var_name {
            VarName::Ident(ident) => {
                quote_spanned! { body.span() =>
                    {
                        {
                            // silence "variable does not need to be mutable"
                            // the extra scope is needed due to:
                            // https://github.com/rust-lang/rust/issues/69663
                            // - otherwise rustc thinks the borrow is carried
                            //   accross the await
                            let _ = &mut #expr;
                        }
                        // if current body has a return, the subsequent bodies will
                        // be marked as unreachable by rustc. Avoid it with a
                        // dummy condition
                        if true {
                            let #ident = #expr.await;
                            // use a dummy binding to always effectively return ()
                            // without appending anything at the end of body
                            // (after all, it could be invalid syntax)
                            let _ = {
                                #body
                            };
                        }
                    }
                }
            }
            VarName::Special => {
                quote_spanned! { body.span() =>
                    let _ = {
                        #body
                    }
                }
            }
        }
    });

    quote! {
        #(#output)*
    }
    .into()
}

#[allow(dead_code)]
fn real_select(input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);
    TokenStream::from(quote! {
        ::futures::select! {
            #input
        }
    })
}

#[proc_macro]
pub fn select(input: TokenStream) -> TokenStream {
    if std::env::var("IS_RUST_ANALYZER").is_ok_and(|v| v != "0" && v != "false") {
        dummy_select_for_ide(input)
    } else {
        real_select(input)
    }
}
