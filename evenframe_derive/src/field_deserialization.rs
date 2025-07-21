use quote::quote;
use syn::{token::Token, Field};

pub fn generate_deserialize(fields: Vec<Field>, struct_name: String) {
    // Generate field deserialization
    let field_deserializations = fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;

        // Check for custom attributes on the field
        let has_validators = field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("evenframe_custom"));

        if has_validators {
            // Generate special handling for this field
            quote! {
                let #field_name: #field_type = map.next_value()?;
                // Apply your custom processing here
            }
        } else {
            // Standard deserialization
            quote! {
                let #field_name: #field_type = map.next_value()?;
            }
        }
    });

    let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();

    let expanded = quote! {
        // Import the trait (adjust path as needed)
        use ::helpers::evenframe::traits::EvenframeDeserialize;

        // Implement your trait with generated logic
        impl<'de> EvenframeDeserialize<'de> for #struct_name {
            fn evenframe_deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                use ::serde::de::{self, Visitor, MapAccess};
                use std::fmt;

                enum Field {
                    #(#field_names,)*
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
                                    #(stringify!(#field_names) => Ok(Field::#field_names),)*
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
                                #(
                                    Field::#field_names => {
                                        if #field_names.is_some() {
                                            return Err(de::Error::duplicate_field(stringify!(#field_names)));
                                        }
                                        #field_names = Some(map.next_value()?);
                                    }
                                )*
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

        // Default Deserialize implementation that delegates to your trait
        impl<'de> ::serde::Deserialize<'de> for #struct_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                Self::evenframe_deserialize(deserializer)
            }
        }


    };
}
