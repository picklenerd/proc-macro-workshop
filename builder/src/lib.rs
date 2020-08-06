extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{Data, Ident, Field, Type, DeriveInput, parse_macro_input};
use quote::{quote, format_ident};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {

    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = format_ident!("{}", input.ident);
    let builder_name = format_ident!("{}Builder", input.ident);

    let fields = match input.data {
        Data::Struct(struct_data) => {
            struct_data.fields
                .iter()
                .map(FieldInfo::from_field)
                .flatten()
                .collect()
        },
        _ => Vec::new(),
    };

    let setters: Vec<proc_macro2::TokenStream> = fields
        .iter()
        .map(FieldInfo::setter)
        .collect();

    let validations: Vec<proc_macro2::TokenStream> = fields
        .iter()
        .map(FieldInfo::validation)
        .collect();

    let field_builders: Vec<proc_macro2::TokenStream> = fields
        .iter()
        .map(FieldInfo::build)
        .collect();

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
            #(#setters)*

            pub fn build(&mut self) -> Result<#struct_name, Box<dyn std::error::Error>> {
                let mut missing_fields = Vec::new();

                #(#validations)*

                if missing_fields.is_empty() {
                    Ok(#struct_name {
                        #(#field_builders)*
                    })
                } else {
                    Err(format!("Missing fields: {}", missing_fields.join(",")).into())
                }
            }
        }
    };

    result.into()
}

struct FieldInfo {
    pub ident: Ident,
    pub inner_type: Type,
    pub is_required: bool,
}

impl FieldInfo {
    pub fn from_field(field: &Field) -> Option<Self> {
        match get_option_type(field) {
            Some((ident, inner_type)) => {
                Some(Self {
                    ident,
                    inner_type,
                    is_required: false,
                })
            }
            None => {
                Some(Self {
                    ident: field.ident.clone()?,
                    inner_type: field.ty.clone(),
                    is_required: true,
                })
            },
        }
    }

    pub fn setter(&self) -> proc_macro2::TokenStream {
        let parameter_name = &self.ident;
        let parameter_type = &self.inner_type;

        quote! {
            pub fn #parameter_name(&mut self, #parameter_name: #parameter_type) -> &mut Self {
                self.#parameter_name = Some(#parameter_name);
                self
            }
        }
    }

    pub fn validation(&self) -> proc_macro2::TokenStream {
        if !self.is_required {
            return proc_macro2::TokenStream::new();
        }

        let parameter_name = &self.ident;

        quote! {
            if self.#parameter_name.is_none() {
                missing_fields.push(stringify!(#parameter_name));
            }
        }
    }

    pub fn build(&self) -> proc_macro2::TokenStream {
        let parameter_name = &self.ident;

        if self.is_required {
            quote! {
                #parameter_name: self.#parameter_name.clone().unwrap(),
            }
        } else {
            quote! {
                #parameter_name: self.#parameter_name.clone(),
            }
        }
    }
}

fn get_option_type(field: &Field) -> Option<(Ident, Type)> {
    use syn::{Path, TypePath, PathArguments, GenericArgument};

    let ident = field.ident.as_ref()?;

    match &field.ty {
        Type::Path(
            TypePath {
                qself: None,
                path: Path {
                    leading_colon: None,
                    segments,
                },
            },
        ) => {
            let segment = segments.iter().next()?;
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    let arg = args.args.iter().next()?;
                    if let GenericArgument::Type(arg_type) = arg {
                        Some((ident.clone(), arg_type.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        },
        _ => None,
    }
}
