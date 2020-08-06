extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    use syn::parse::Parse;

    let input = parse_macro_input!(input as DeriveInput);

    unimplemented!()
}
