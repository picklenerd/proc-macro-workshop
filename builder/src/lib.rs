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

        impl #builder_name {
            pub fn executable(&mut self, executable: String) -> &mut Self {
                self.executable = Some(executable);
                self
            }

            pub fn args(&mut self, args: Vec<String>) -> &mut Self {
                self.args = Some(args);
                self
            }

            pub fn env(&mut self, env: Vec<String>) -> &mut Self {
                self.env = Some(env);
                self
            }

            pub fn current_dir(&mut self, current_dir: String) -> &mut Self {
                self.current_dir = Some(current_dir);
                self
            }

            pub fn build(&mut self) -> Result<#struct_name, Box<dyn std::error::Error>> {
                match (self.executable.clone(), self.args.clone(), self.env.clone(), self.current_dir.clone()) {
                    (None, _, _, _) => Err("Missing field: executable".into()),
                    (_, None, _, _) => Err("Missing field: args".into()),
                    (_, _, None, _) => Err("Missing field: env".into()),
                    (_, _, _, None) => Err("Missing field: current_dir".into()),
                    (Some(executable), Some(args), Some(env), Some(current_dir)) => Ok(#struct_name {
                        executable,
                        args,
                        env,
                        current_dir,
                    }),
                }
            }
        }
    };

    result.into()
}
