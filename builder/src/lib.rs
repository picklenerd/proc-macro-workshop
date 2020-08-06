extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{Data, Ident, Field, Type, DeriveInput, parse_macro_input};
use quote::{quote, format_ident};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // eprintln!("Input: {:#?}", &input);

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

    let field_definitions = data_from_fields(&fields, FieldInfo::field_definition);
    let default_builders = data_from_fields(&fields, FieldInfo::default_builder);
    let setters = data_from_fields(&fields, FieldInfo::setter);
    let validations = data_from_fields(&fields, FieldInfo::validation);
    let field_builders = data_from_fields(&fields, FieldInfo::build);

    let result = quote! {
        pub struct #builder_name {
            #(#field_definitions)*
        }

        impl #struct_name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #(#default_builders)*
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

    // eprintln!("Output: {:#?}", &result);

    result.into()
}

fn data_from_fields<F>(fields: &[FieldInfo], data_extractor: F) -> Vec<proc_macro2::TokenStream> 
where F: Fn(&FieldInfo) -> proc_macro2::TokenStream
{
    fields
        .iter()
        .map(data_extractor)
        .collect()
}

struct StructInfo {
    ident: Ident,
    fields: Vec<FieldInfo>,
}

struct FieldInfo {
    pub ident: Ident,
    pub inner_type: Type,
    pub is_required: bool,
    pub attributes: Vec<AttributeInfo>
}

impl FieldInfo {
    pub fn from_field(field: &Field) -> Option<Self> {
        match option_info(field) {
            Some((ident, inner_type)) => {
                Some(Self {
                    ident,
                    inner_type,
                    is_required: false,
                    attributes: Vec::new(),
                })
            }
            None => {
                Some(Self {
                    ident: field.ident.clone()?,
                    inner_type: field.ty.clone(),
                    is_required: true,
                    attributes: Vec::new(),
                })
            },
        }
    }

    pub fn field_definition(&self) -> proc_macro2::TokenStream {
        let parameter_name = &self.ident;
        let parameter_type = &self.inner_type;

        quote! {
            #parameter_name: Option<#parameter_type>,
        }    
    }

    pub fn default_builder(&self) -> proc_macro2::TokenStream {
        let parameter_name = &self.ident;

        quote! {
            #parameter_name: None,
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

struct AttributeInfo {
    ident: Ident,
    tag: Ident,
    value: String,
}

fn option_info(field: &Field) -> Option<(Ident, Type)> {
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