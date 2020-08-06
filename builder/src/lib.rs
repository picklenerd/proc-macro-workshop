extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};
use quote::{quote, format_ident};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {

    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = format_ident!("{}", input.ident);
    let builder_name = format_ident!("{}Builder", input.ident);

    let result = quote! {
        pub struct #builder_name {
            executable: Option<String>,
            args: Option<Vec<String>>,
            env: Option<Vec<String>>,
            current_dir: Option<String>,
        }

        impl #struct_name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    executable: None,
                    args: None,
                    env: None,
                    current_dir: None,
                }
            }

        }
    };

    result.into()
}


            // pub fn executable(&mut self, executable: String) -> &mut Self {
            //     self.executable = Some(executable);
            //     self
            // }