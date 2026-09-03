//! Define ad-hoc polymorphic functions using a macro.
//!
//! Rust only exposes parametric polymorphism for functions, but traits are the
//! simplest way to get ad-hoc polymorphism. The [`polymorphic`] macro is for
//! convenience. Using very clean syntax, it can generate a trait, detect errors
//! in variants, and produce a usable function that is generic over all inputs and
//! outputs.

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenTree};
use quote::{format_ident, quote};
use syn::{
    Attribute, Block, Error, Pat, Result, Token, Type, braced, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
};

struct PolymorphicFn {
    attrs: Vec<Attribute>,
    qualifiers: proc_macro2::TokenStream,
    name: Ident,
    args: Vec<Pat>,
    variants: Vec<Variant>,
}

impl Parse for PolymorphicFn {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let mut qualifiers = proc_macro2::TokenStream::new();

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

            variants_content.parse::<Token![=>]>()?;

            let body: Block = variants_content.parse()?;

            let variant = Variant {
                ty,
                args: variant_args,
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
            variants,
        })
    }
}

struct Variant {
    ty: Type,
    args: Vec<Type>,
    body: Block,
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
/// Internally uses a trait.
///
/// # Safety
///
/// This lets you define unsafe functions, even with `#[forbid(unsafe_code)]`.
/// Not actually unsound, it won't let you do anything unsafe in safe code still.
#[proc_macro]
pub fn polymorphic(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Polymorphic);

    let functions = input.functions.iter().map(expand_function);

    quote! {
        #(#functions)*
    }
    .into()
}

fn expand_function(function: &PolymorphicFn) -> proc_macro2::TokenStream {
    let PolymorphicFn {
        attrs,
        qualifiers,
        name,
        args,
        variants,
    } = function;

    let trait_name = format_ident!("__{}_trait_internal", name);
    let method_name = format_ident!("__{}_internal", name);

    let mut arg_names = Vec::with_capacity(args.len());

    for arg in args {
        match arg {
            Pat::Ident(pat) => arg_names.push(pat.ident.clone()),

            pat => {
                return Error::new(
                    pat.span(),
                    "polymorphic function arguments must be identifiers",
                )
                .into_compile_error();
            }
        }
    }

    let impls = variants.iter().map(|variant| {
        let ty = &variant.ty;
        let variant_args = &variant.args;
        let body = &variant.body;

        quote! {
            impl #trait_name<#(#variant_args),*> for #ty {
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
        trait #trait_name<#(#arg_names),*> {
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
        #qualifiers fn #name<#(#arg_names,)* V>(
            #(
                #arg_names: #arg_names
            ),*
        ) -> V
        where
            V: #trait_name<#(#arg_names),*>,
        {
            <V as #trait_name<#(#arg_names),*>>::
                #method_name(#(#arg_names),*)
        }
    }
}
