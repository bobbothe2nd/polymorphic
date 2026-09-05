//! Define ad-hoc polymorphic functions using a macro.
//!
//! Rust only exposes parametric polymorphism for functions, but traits are the
//! simplest way to get ad-hoc polymorphism. The [`polymorphic`] macro is for
//! convenience. Using very clean syntax, it can generate a trait, detect errors
//! in variants, and produce a usable function that is generic over all inputs and
//! outputs.

use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Ident, Span, TokenTree, TokenStream};
use quote::{format_ident, quote};
use syn::{
    Attribute, Block, Error, Expr, Pat, Result, Token, Type, braced, bracketed, parenthesized, parse::{Parse, ParseStream}, parse_macro_input, parse_quote, punctuated::Punctuated, spanned::Spanned, token::Bracket,
};

struct PolymorphicFn {
    attrs: Vec<Attribute>,
    qualifiers: TokenStream,
    name: Ident,
    args: Vec<Pat>,
    const_types: Vec<Type>,
    variants: Vec<Variant>,
}

impl Parse for PolymorphicFn {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let mut qualifiers = TokenStream::new();

        while !input.peek(Token![fn]) {
            if input.peek(Token![const]) {
                return Err(Error::new(
                    Span::call_site(),
                    "polymorphic functions cannot be const",
                ));
            }

            let token: TokenTree = input.parse()?;
            qualifiers.extend(std::iter::once(token));
        }

        input.parse::<Token![fn]>()?;
        let name: Ident = input.parse()?;

        let args_content;
        parenthesized!(args_content in input);

        let fn_args =
            Punctuated::<Pat, Token![,]>::parse_terminated_with(&args_content, Pat::parse_single)?
                .into_iter()
                .collect::<Vec<_>>();

        let const_types = if input.peek(Bracket) {
            let const_types;
            bracketed!(const_types in input);

            Punctuated::<Type, Token![,]>::parse_terminated(&const_types)?
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        input.parse::<Token![->]>()?;
        input.parse::<Token![match]>()?;

        let variants_content;
        braced!(variants_content in input);

        let mut variants = Vec::new();

        while !variants_content.is_empty() {
            let ty: Type = variants_content.parse()?;

            let args_content;
            parenthesized!(args_content in variants_content);

            let variant_args = Punctuated::<Type, Token![,]>::parse_terminated(&args_content)?
                .into_iter()
                .collect::<Vec<_>>();

            if variant_args.len() != fn_args.len() {
                return Err(Error::new(
                    ty.span(),
                    format!(
                        "expected {} argument types, found {}",
                        fn_args.len(),
                        variant_args.len()
                    ),
                ));
            }

            let consts = if variants_content.peek(Bracket) {
                let consts;
                bracketed!(consts in variants_content);

                Punctuated::<Expr, Token![,]>::parse_terminated(&consts)?
                    .into_iter()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };

            if consts.len() != const_types.len() {
                return Err(Error::new(
                    ty.span(),
                    format!(
                        "expected {} constant values, found {}",
                        const_types.len(),
                        consts.len()
                    ),
                ));
            }

            variants_content.parse::<Token![=>]>()?;

            let body: Block = variants_content.parse()?;

            let variant = Variant {
                ty,
                args: variant_args,
                consts,
                body,
            };

            if variants.contains(&variant) {
                return Err(Error::new(
                    variant.ty.span(),
                    "duplicate definition of variant",
                ));
            }

            variants.push(variant);
        }

        Ok(Self {
            attrs,
            qualifiers,
            name,
            args: fn_args,
            const_types,
            variants,
        })
    }
}

struct Variant {
    ty: Type,
    args: Vec<Type>,
    body: Block,
    consts: Vec<Expr>,
}

impl PartialEq for Variant {
    fn eq(&self, other: &Self) -> bool {
        let a_ty = &self.ty;
        let b_ty = &other.ty;

        quote!(#a_ty).to_string() == quote!(#b_ty).to_string()
            && self
                .args
                .iter()
                .map(|ty| quote!(#ty).to_string())
                .eq(other.args.iter().map(|ty| quote!(#ty).to_string()))
    }
}

struct Polymorphic {
    functions: Vec<PolymorphicFn>,
}

impl Parse for Polymorphic {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut functions = Vec::new();

        while !input.is_empty() {
            functions.push(input.parse()?);
        }

        Ok(Self { functions })
    }
}

/// Defines polymorphic functions by return type and input type.
///
/// Only for convenience and internally creates a private, hidden trait that implements all the different rules:
///
/// ```rust
/// polymorphic::polymorphic! {
///     #[must_use]
///     fn foo(a, b) -> match {
///         usize (u32, u64) => { ((a as u64) + b) as usize }
///         usize (u64, u32) => { (a - (b as u64)) as usize }
///     }
///
///     #[must_use]
///     pub fn zero() -> match {
///         u32 () => { 0 }
///         i32 () => { 0 }
///         f32 () => { 0.0 }
///         f64 () => { 0.0 }
///     }
/// }
///
/// fn main() {
///     let val: usize = foo(3, 2u32);
///     assert_eq!(val, 1);
///     let val: usize = foo(3u32, 2);
///     assert_eq!(val, 5);
///     let val: u32 = zero();
///     assert_eq!(val, 0);
/// }
/// ```
///
/// You can also specify constants:
/// 
/// ```rust
/// polymorphic! {
///     fn bar(a, b) [usize, usize] -> match {
///         usize (u32, u64) [3, N] => {
///             const {
///                 assert!(N == 2);
///             }
/// 
///             ((a as u64) + b) as usize
///         }
///         [u8; N] (u64, [u8; N]) [N, 0] => { b }
///         usize (u64, u32) [_, 4] => { (a - (b as u64)) as usize }
///         usize (u32, u32) [2, 4] => { (a - b) as usize }
///     }
/// }
/// ```
///
/// Internally uses a trait.
///
/// # Safety
///
/// This lets you define unsafe functions, even with `#[forbid(unsafe_code)]`.
/// Not actually unsound, it won't let you do anything unsafe in safe code still.
#[proc_macro]
pub fn polymorphic(input: TokenStream1) -> TokenStream1 {
    let mut input = parse_macro_input!(input as Polymorphic);

    let functions = input.functions.iter_mut().map(expand_function);

    quote! {
        #(#functions)*
    }
    .into()
}

fn expand_function(function: &mut PolymorphicFn) -> TokenStream {
    let PolymorphicFn {
        attrs,
        qualifiers,
        name,
        args,
        const_types,
        variants,
    } = function;

    let trait_name = format_ident!("__{}_trait_internal", name);
    let method_name = format_ident!("__{}_internal", name);

    let mut arg_names = Vec::with_capacity(args.len());

    for arg in args {
        match arg {
            Pat::Ident(pat) => arg_names.push(pat.ident.clone()),

            pat => return Error::new(
                pat.span(),
                "polymorphic function arguments must be identifiers",
            )
            .into_compile_error(),
        }
    }

    let mut const_names = Vec::with_capacity(const_types.len());

    let start = b'A' as usize + arg_names.len();

    for (i, b) in (start..(start + const_types.len())).enumerate() {
        match str::from_utf8(&[b'_', b'_', b as u8]) {
            Ok(char) => const_names.push(Ident::new(char, const_types[i].span())),
            Err(_) => return Error::new(
                Span::call_site(),
                "too many polymorphic constants"
            ).into_compile_error(),
        }
    }

    let impls = variants.iter_mut().map(|variant| {
        let ty = &variant.ty;
        let variant_args = &variant.args;
        let variant_consts = &mut variant.consts;
        let body = &variant.body;

        let mut impl_consts = Vec::new();
    
        for (i, expr) in variant_consts.iter_mut().enumerate() {
            match expr {
                Expr::Infer(_) => {
                    let name = &const_names[i];
                    let ty = &const_types[i];

                    impl_consts.push(quote!(const #name: #ty));
                    *expr = parse_quote!(#name);
                }

                Expr::Path(path) if path.path.segments.len() == 1 => {
                    let name = &path.path.segments[0].ident;
                    let ty = &const_types[i];

                    impl_consts.push(quote!(const #name: #ty));
                }

                _ => {}
            }
        }

        quote! {
            impl<#(#impl_consts),*> #trait_name<#(#variant_consts,)* #(#variant_args),*> for #ty {
                #[inline]
                #[allow(unused_variables)]
                fn #method_name(
                    #(
                        #arg_names: #variant_args
                    ),*
                ) -> Self #body
            }
        }
    });

    quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        trait #trait_name<
            #(
                const #const_names: #const_types,
            )*
            #(
                #arg_names,
            )*
        > {
            fn #method_name(
                #(
                    #arg_names: #arg_names
                ),*
            ) -> Self;
        }

        #(#impls)*

        #(#attrs)*
        #[allow(private_bounds)]
        #[allow(non_camel_case_types)]
        #qualifiers fn #name<
            #(
                const #const_names: #const_types,
            )*
            #(
                #arg_names,
            )*
            V
        >(
            #(
                #arg_names: #arg_names
            ),*
        ) -> V
        where
            V: #trait_name<
                #(
                    #const_names,
                )*
                #(
                    #arg_names,
                )*
            >,
        {
            <V as #trait_name<
                #(
                    #const_names,
                )*
                #(
                    #arg_names,
                )*
            >>::#method_name(#(#arg_names),*)
        }
    }
}
