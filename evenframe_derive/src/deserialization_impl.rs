use crate::validator_parser::parse_field_validators_with_logic;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

pub fn generate_custom_deserialize(input: &DeriveInput) -> proc_macro2::TokenStream {
    let struct_name = &input.ident;

    // Extract fields from the struct
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return quote! {},
        },
        _ => return quote! {},
    };

    // Generate field deserialization with validation
    let field_deserializations = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().expect(&format!(
            "Something went wrong getting the syn::Ident for this field: {:#?}",
            field
        ));
        let field_type = &field.ty;
        let enum_variant = quote::format_ident!("{}", to_pascal_case(&field_name.to_string()));

        // Create a temporary variable name for validation
        let temp_var_name = format!("__temp_{}", field_name);

        // Parse validators and get both validator tokens and logic tokens
        let (_, validation_logic_tokens) =
            match parse_field_validators_with_logic(&field.attrs, &temp_var_name) {
                Ok(tokens) => tokens,
                Err(err) => return err.to_compile_error(),
            };

        if !validation_logic_tokens.is_empty() {
            let temp_var = quote::format_ident!("{}", temp_var_name);
            // Generate validation code
            quote! {
                Field::#enum_variant => {
                    if #field_name.is_some() {
                        return Err(de::Error::duplicate_field(stringify!(#field_name)));
                    }
                    let mut #temp_var: #field_type = map.next_value()?;
                    // Apply validators
                    #(#validation_logic_tokens)*
                    #field_name = Some(#temp_var);
                }
            }
        } else {
            // Standard deserialization without validation
            quote! {
                Field::#enum_variant => {
                    if #field_name.is_some() {
                        return Err(de::Error::duplicate_field(stringify!(#field_name)));
                    }
                    #field_name = Some(map.next_value()?);
                }
            }
        }
    });

    let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let enum_variants: Vec<_> = fields
        .iter()
        .map(|f| {
            let name = f.ident.as_ref().expect(&format!(
                "Something went wrong getting the syn::Ident for this field: {:#?}",
                f
            ));
            quote::format_ident!("{}", to_pascal_case(&name.to_string()))
        })
        .collect();

    quote! {
        // Import the trait
        use ::helpers::evenframe::traits::EvenframeDeserialize;

        // Custom deserialization implementation
        impl<'de> EvenframeDeserialize<'de> for #struct_name {
            fn evenframe_deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                use ::serde::de::{self, Visitor, MapAccess};
                use std::fmt;

                enum Field {
                    #(#enum_variants,)*
                }

                impl<'de> ::serde::Deserialize<'de> for Field {
                    fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
                    where
                        D: ::serde::Deserializer<'de>,
                    {
                        struct FieldVisitor;

                        impl<'de> Visitor<'de> for FieldVisitor {
                            type Value = Field;

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_str("field identifier")
                            }

                            fn visit_str<E>(self, value: &str) -> Result<Field, E>
                            where
                                E: de::Error,
                            {
                                match value {
                                    #(stringify!(#field_names) => Ok(Field::#enum_variants),)*
                                    _ => Err(de::Error::unknown_field(value, &[#(stringify!(#field_names)),*])),
                                }
                            }
                        }

                        deserializer.deserialize_identifier(FieldVisitor)
                    }
                }

                struct StructVisitor;

                impl<'de> Visitor<'de> for StructVisitor {
                    type Value = #struct_name;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str(concat!("struct ", stringify!(#struct_name)))
                    }

                    fn visit_map<V>(self, mut map: V) -> Result<#struct_name, V::Error>
                    where
                        V: MapAccess<'de>,
                    {
                        #(let mut #field_names = None;)*

                        while let Some(key) = map.next_key()? {
                            match key {
                                #(#field_deserializations)*
                            }
                        }

                        #(
                            let #field_names = #field_names.ok_or_else(|| de::Error::missing_field(stringify!(#field_names)))?;
                        )*

                        Ok(#struct_name {
                            #(#field_names,)*
                        })
                    }
                }

                const FIELDS: &'static [&'static str] = &[#(stringify!(#field_names)),*];
                deserializer.deserialize_struct(stringify!(#struct_name), FIELDS, StructVisitor)
            }
        }

        // Default Deserialize implementation that delegates to custom trait
        impl<'de> ::serde::Deserialize<'de> for #struct_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                Self::evenframe_deserialize(deserializer)
            }
        }
    }
}
