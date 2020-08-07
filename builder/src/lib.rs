extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{Data, Attribute, Ident, Field, Type, DeriveInput, parse_macro_input};
use quote::{quote, format_ident};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = format_ident!("{}", input.ident);
    let builder_name = format_ident!("{}Builder", input.ident);

    let struct_info = StructInfo::from(&input);

    let field_definitions = data_from_fields(&struct_info.fields, FieldInfo::field_definition);
    let default_builders = data_from_fields(&struct_info.fields, FieldInfo::default_builder);
    let setters = data_from_fields(&struct_info.fields, FieldInfo::setter);
    let validations = data_from_fields(&struct_info.fields, FieldInfo::validation);
    let each_builders = data_from_fields(&struct_info.fields, FieldInfo::each);
    let field_builders = data_from_fields(&struct_info.fields, FieldInfo::build);

    for builder in &each_builders {
        println!("{}", builder);
    }
    println!("--");

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

            #(#setters)*

            #(#each_builders)*
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

#[derive(Debug)]
struct StructInfo {
    pub ident: Ident,
    pub fields: Vec<FieldInfo>,
}

impl From<&DeriveInput> for StructInfo {
    fn from(input: &DeriveInput) -> Self {
        let fields = match &input.data {
            Data::Struct(struct_data) => {
                struct_data.fields
                    .iter()
                    .map(FieldInfo::new)
                    .flatten()
                    .collect()
            },
            _ => Vec::new(),
        };

        Self {
            ident: input.ident.clone(),
            fields,
        }
    }
}

#[derive(Debug)]
enum SpecialField {
    Vec(Type),
    Option(Type),
}

#[derive(Debug)]
struct FieldInfo {
    pub ident: Ident,
    pub ty: Type,
    pub special_field: Option<SpecialField>,
    pub attributes: Vec<AttributeInfo>
}

impl FieldInfo {
    pub fn new(field: &Field) -> Option<Self> {
        let attributes = field.attrs
            .iter()
            .map(AttributeInfo::new)
            .flatten()
            .collect();

        match &field.ident {
            Some(ident) => {
                Some(Self {
                    ident: ident.clone(),
                    ty: field.ty.clone(),
                    special_field: special_field_info(&field),
                    attributes,
                })        
            },
            None => None,
        }
    }

    pub fn field_definition(&self) -> proc_macro2::TokenStream {
        let parameter_name = &self.ident;
        let parameter_type = &self.ty;

        quote! {
            #parameter_name: Option<#parameter_type>,
        }    
    }

    pub fn default_builder(&self) -> proc_macro2::TokenStream {
        let parameter_name = &self.ident;

        match self.special_field {
            Some(SpecialField::Vec(_)) => quote! {
                #parameter_name: Vec::new(),
            },
            _ => quote! {
                #parameter_name: None,
            } 
        }
    }

    pub fn setter(&self) -> proc_macro2::TokenStream {
        let parameter_name = &self.ident;
        let parameter_type = &self.ty;

        match &self.special_field  {
            Some(SpecialField::Vec(_)) => proc_macro2::TokenStream::new(),
            _ => {
                quote! {
                    pub fn #parameter_name(&mut self, #parameter_name: #parameter_type) -> &mut Self {
                        self.#parameter_name = Some(#parameter_name);
                        self
                    }
                }
            },
        }
    }

    pub fn validation(&self) -> proc_macro2::TokenStream {
        match &self.special_field {
            Some(SpecialField::Option(_)) => {
                let parameter_name = &self.ident;

                quote! {
                    if self.#parameter_name.is_none() {
                        missing_fields.push(stringify!(#parameter_name));
                    }
                }
            },
            _ => proc_macro2::TokenStream::new(),
        }
    }

    pub fn each(&self) -> proc_macro2::TokenStream {
        match &self.special_field {
            Some(SpecialField::Vec(inner_type)) => {
                let parameter_name = &self.ident;
        
                let each_attr = &self.attributes
                    .iter()
                    .find(|attr| attr.tag == "each");

                match each_attr {
                    Some(each_attr) => {
                        let function_name = format_ident!("{}", &each_attr.value);

                        quote! {
                            pub fn #function_name(&mut self, #function_name: #inner_type) -> &mut Self {
                                self.#parameter_name.push(#function_name);
                                self
                            }
                        }
                    },
                    None => proc_macro2::TokenStream::new(),
                }
            },
            _ => proc_macro2::TokenStream::new(),
        }
    }

    pub fn build(&self) -> proc_macro2::TokenStream {
        let parameter_name = &self.ident;

        if let Some(SpecialField::Option(_)) = self.special_field {
            quote! {
                #parameter_name: self.#parameter_name.clone(),
            }
        } else {
            quote! {
                #parameter_name: self.#parameter_name.clone().unwrap(),
            }
        }
    }
}

#[derive(Debug)]
struct AttributeInfo {
    pub ident: Ident,
    pub tag:  Ident,
    pub value: String,
}

impl AttributeInfo {
    pub fn new(attribute: &Attribute) -> Option<Self> {
        use proc_macro2::TokenTree;
        use syn::PathSegment;

        let ident = match &attribute.path.segments.iter().next() {
            Some(PathSegment { ident, .. }) => ident.clone(),
            _ => return None,
        };

        let (tag, value) = match attribute.tokens.clone().into_iter().next() {
            Some(TokenTree::Group(group)) => {
                let tokens: Vec<TokenTree> = group
                    .stream()
                    .into_iter()
                    .collect();

                match &tokens.as_slice() {
                    &[TokenTree::Ident(ident), TokenTree::Punct(punct), TokenTree::Literal(literal)] => {
                        if punct.as_char() == '=' {
                            (ident.clone(), literal.to_string().replace("\"", ""))
                        } else {
                            return None
                        }
                    },
                    _ => return None,
                }
            }
            _ => return None,
        };

        Some(Self {
            ident,
            tag,
            value,
        })
    }
}

fn special_field_info(field: &Field) -> Option<SpecialField> {
    use syn::{Path, TypePath, PathArguments, GenericArgument};

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
                        Some(SpecialField::Option(arg_type.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else if segment.ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    let arg = args.args.iter().next()?;
                    if let GenericArgument::Type(arg_type) = arg {
                        Some(SpecialField::Vec(arg_type.clone()))
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
