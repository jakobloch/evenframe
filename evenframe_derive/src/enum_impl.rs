use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, LitStr, spanned::Spanned};

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
                        if fields.unnamed.is_empty() {
                            return syn::Error::new(
                                fields.span(),
                                format!("Variant '{}' has unnamed fields but no actual fields defined.\n\nExample of valid unnamed variant:\n{}(String)\n{}(i32, String)", 
                                    variant_name, variant_name, variant_name)
                            ).to_compile_error();
                        } else if fields.unnamed.len() == 1 {
                            let field = &fields.unnamed[0];
                            let field_type = parse_data_type(&field.ty);
                            quote! { Some(::helpers::evenframe::types::VariantData::DataStructureRef(#field_type)) }
                        } else {
                            let field_types =
                                fields.unnamed.iter().map(|f| parse_data_type(&f.ty));
                            quote! { Some(::helpers::evenframe::types::VariantData::DataStructureRef(::helpers::evenframe::types::FieldType::Tuple(vec![ #(#field_types),* ]))) }
                        }
                    }
                    syn::Fields::Named(fields) => {
                        match generate_struct_fields_tokens(&variant_name, fields) {
                            Ok(struct_fields) => {
                                quote! { 
                                    Some(::helpers::evenframe::types::VariantData::InlineStruct(
                                        ::helpers::evenframe::types::StructConfig {
                                            name: #variant_name.to_string(),
                                            fields: vec![ #(#struct_fields),* ],
                                            validators: vec![],
                                        }
                                    ))
                                }
                            }
                            Err(err) => return err.to_compile_error(),
                        }
                    }
                };

                quote! {
                    ::helpers::evenframe::types::Variant {
                        name: #variant_name.to_string(),
                        data: #data_tokens,
                    }
                }
            })
            .collect();

        let enum_impl = quote! {
            impl #ident {
                pub fn variants() -> ::helpers::evenframe::types::TaggedUnion {
                    ::helpers::evenframe::types::TaggedUnion {
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
                        match generate_struct_fields_tokens(&variant_name, fields) {
                            Ok(struct_fields) => {
                                Some(quote! {
                                    ::helpers::evenframe::types::StructConfig {
                                        name: #variant_name.to_string(),
                                        fields: vec![ #(#struct_fields),* ],
                                        validators: vec![],
                                    }
                                })
                            }
                            Err(err) => Some(err.to_compile_error()),
                        }
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

                fn variants() -> Vec<::helpers::evenframe::types::Variant> {
                    vec![ #(#variant_tokens),* ]
                }

                fn inline_structs() -> Option<Vec<::helpers::evenframe::types::StructConfig>> {
                    #inline_structs_impl
                }

                fn tagged_union() -> ::helpers::evenframe::types::TaggedUnion {
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
            format!("The Evenframe derive macro can only be applied to enums when using generate_enum_impl.\n\nYou tried to apply it to: {}\n\nExample of correct usage:\n#[derive(Evenframe)]\nenum MyEnum {{\n    Variant1,\n    Variant2(String),\n    Variant3 {{ field: i32 }}\n}}", ident),
        )
        .to_compile_error()
        .into()
    }
}

/// Helper function to generate struct field tokens for named fields
/// This avoids duplication between variant processing and inline struct collection
fn generate_struct_fields_tokens(
    variant_name: &str,
    fields: &syn::FieldsNamed,
) -> Result<Vec<proc_macro2::TokenStream>, syn::Error> {
    if fields.named.is_empty() {
        return Err(syn::Error::new(
            fields.span(),
            format!("Variant '{}' has named fields but no actual fields defined.\n\nExample of valid named variant:\n{} {{\n    field1: String,\n    field2: i32,\n}}", 
                variant_name, variant_name)
        ));
    }
    
    fields.named
        .iter()
        .map(|f| {
            let field_ident = f.ident.as_ref()
                .ok_or_else(|| {
                    syn::Error::new(
                        f.span(),
                        "Internal error: Named field should have an identifier"
                    )
                })?;
            let field_name = field_ident.to_string();
            let field_type = parse_data_type(&f.ty);
            
            // Parse field attributes (format, validators, etc.)
            let format = parse_format_attribute(&f.attrs).map_err(|err| {
                syn::Error::new(
                    f.span(),
                    format!("Failed to parse format attribute for field '{}' in variant '{}': {}",
                        field_name, variant_name, err)
                )
            })?;
            
            let validators = parse_field_validators(&f.attrs).map_err(|err| {
                syn::Error::new(
                    f.span(),
                    format!("Failed to parse validators for field '{}' in variant '{}': {}\n\nExample usage:\n#[validate(min_length = 3)]\nfield_name: String",
                        field_name, variant_name, err)
                )
            })?;
            
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
            
            Ok(quote! {
                ::helpers::evenframe::types::StructField {
                    field_name: #field_name.to_string(),
                    field_type: #field_type,
                    edge_config: None,
                    define_config: None,
                    format: #format_tokens,
                    validators: #validators_tokens,
                    always_regenerate: false
                }
            })
        })
        .collect()
}