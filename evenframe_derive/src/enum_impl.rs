use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, LitStr};

use crate::attributes::parse_format_attribute;
use crate::type_parser::parse_data_type;
use crate::validator_parser::parse_field_validators;

pub fn generate_enum_impl(input: DeriveInput) -> TokenStream {
    let ident = input.ident;
    
    if let Data::Enum(ref data_enum) = input.data {
        let enum_name_lit = LitStr::new(&ident.to_string(), ident.span());
        let variant_tokens: Vec<_> = data_enum
            .variants
            .iter()
            .map(|variant| {
                let variant_name = variant.ident.to_string();
                let data_tokens = match &variant.fields {
                    syn::Fields::Unit => quote! { None },
                    syn::Fields::Unnamed(fields) => {
                        if fields.unnamed.len() == 1 {
                            let ty = &fields.unnamed.first().unwrap().ty;
                            let field_type = parse_data_type(ty);
                            quote! { Some(::helpers::evenframe::schemasync::VariantData::DataStructureRef(#field_type)) }
                        } else {
                            let field_types =
                                fields.unnamed.iter().map(|f| parse_data_type(&f.ty));
                            quote! { Some(::helpers::evenframe::schemasync::VariantData::DataStructureRef(::helpers::evenframe::schemasync::FieldType::Tuple(vec![ #(#field_types),* ]))) }
                        }
                    }
                    syn::Fields::Named(fields) => {
                        // Create an inline struct for named fields
                        let struct_name = variant_name.clone();
                        let struct_fields = fields.named.iter().map(|f| {
                            let field_name = f.ident.as_ref().unwrap().to_string();
                            let field_type = parse_data_type(&f.ty);
                            
                            // Parse field attributes (format, validators, etc.)
                            let format = parse_format_attribute(&f.attrs);
                            let validators = match parse_field_validators(&f.attrs) {
                                Ok(v) => v,
                                Err(err) => return err.to_compile_error(),
                            };
                            
                            let format_tokens = if let Some(ref fmt) = format {
                                quote! { Some(#fmt) }
                            } else {
                                quote! { None }
                            };
                            
                            let validators_tokens = if validators.is_empty() {
                                quote! { vec![] }
                            } else {
                                quote! { vec![#(#validators),*] }
                            };
                            
                            quote! {
                                ::helpers::evenframe::schemasync::StructField {
                                    field_name: #field_name.to_string(),
                                    field_type: #field_type,
                                    edge_config: None,
                                    define_config: None,
                                    format: #format_tokens,
                                    validators: #validators_tokens,
                                    always_regenerate: false
                                }
                            }
                        });
                        
                        quote! { 
                            Some(::helpers::evenframe::schemasync::VariantData::InlineStruct(
                                ::helpers::evenframe::schemasync::StructConfig {
                                    name: #struct_name.to_string(),
                                    fields: vec![ #(#struct_fields),* ],
                                    validators: vec![],
                                }
                            ))
                        }
                    }
                };

                quote! {
                    ::helpers::evenframe::schemasync::Variant {
                        name: #variant_name.to_string(),
                        data: #data_tokens,
                    }
                }
            })
            .collect();

        let enum_impl = quote! {
            impl #ident {
                pub fn variants() -> ::helpers::evenframe::schemasync::TaggedUnion {
                    ::helpers::evenframe::schemasync::TaggedUnion {
                        enum_name: #enum_name_lit.to_string(),
                        variants: vec![ #(#variant_tokens),* ],
                    }
                }
            }
        };

        // Collect inline structs from variants with named fields
        let inline_structs_tokens: Vec<_> = data_enum
            .variants
            .iter()
            .filter_map(|variant| {
                match &variant.fields {
                    syn::Fields::Named(fields) => {
                        let variant_name = variant.ident.to_string();
                        let struct_name = variant_name.clone();
                        let struct_fields = fields.named.iter().map(|f| {
                            let field_name = f.ident.as_ref().unwrap().to_string();
                            let field_type = parse_data_type(&f.ty);
                            
                            // Parse field attributes (format, validators, etc.)
                            let format = parse_format_attribute(&f.attrs);
                            let validators = match parse_field_validators(&f.attrs) {
                                Ok(v) => v,
                                Err(err) => return err.to_compile_error(),
                            };
                            
                            let format_tokens = if let Some(ref fmt) = format {
                                quote! { Some(#fmt) }
                            } else {
                                quote! { None }
                            };
                            
                            let validators_tokens = if validators.is_empty() {
                                quote! { vec![] }
                            } else {
                                quote! { vec![#(#validators),*] }
                            };
                            
                            quote! {
                                ::helpers::evenframe::schemasync::StructField {
                                    field_name: #field_name.to_string(),
                                    field_type: #field_type,
                                    edge_config: None,
                                    define_config: None,
                                    format: #format_tokens,
                                    validators: #validators_tokens,
                                    always_regenerate: false
                                }
                            }
                        });
                        
                        Some(quote! {
                            ::helpers::evenframe::schemasync::StructConfig {
                                name: #struct_name.to_string(),
                                fields: vec![ #(#struct_fields),* ],
                                validators: vec![],
                            }
                        })
                    }
                    _ => None
                }
            })
            .collect();
        
        let inline_structs_impl = if inline_structs_tokens.is_empty() {
            quote! { None }
        } else {
            quote! { Some(vec![ #(#inline_structs_tokens),* ]) }
        };

        // Generate EvenframeEnum trait implementation
        let evenframe_enum_impl = quote! {
            impl ::helpers::evenframe::traits::EvenframeEnum for #ident {
                fn name() -> String {
                    #enum_name_lit.to_string()
                }

                fn variants() -> Vec<::helpers::evenframe::schemasync::Variant> {
                    vec![ #(#variant_tokens),* ]
                }

                fn inline_structs() -> Option<Vec<::helpers::evenframe::schemasync::StructConfig>> {
                    #inline_structs_impl
                }

                fn tagged_union() -> ::helpers::evenframe::schemasync::TaggedUnion {
                    #ident::variants()
                }
            }
        };

        let output = quote! {
            // Import the trait so it's available for method calls
            use ::helpers::evenframe::traits::EvenframeEnum as _;

            #enum_impl
            #evenframe_enum_impl
        };

        output.into()
    } else {
        syn::Error::new(
            ident.span(),
            "generate_enum_impl called on non-enum",
        )
        .to_compile_error()
        .into()
    }
}