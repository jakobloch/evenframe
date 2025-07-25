use quote::quote;
use syn::spanned::Spanned;
use syn::{GenericArgument, PathArguments, Type};

/// Generate a helpful error message for unsupported types
fn unsupported_type_error(ty: &Type, type_str: &str, hint: &str) -> proc_macro2::TokenStream {
    syn::Error::new(
        ty.span(),
        format!(
            "Unsupported type: '{}'. {}\n\nSupported types include:\n\
            - Primitives: bool, char, String, i8-i128, u8-u128, f32, f64\n\
            - Special: Decimal, DateTime, Duration, EvenframeRecordId\n\
            - Containers: Option<T>, Vec<T>, HashMap<K,V>, BTreeMap<K,V>\n\
            - Custom: RecordLink<T>, or any custom struct/enum",
            type_str, hint
        ),
    )
    .to_compile_error()
}

/// Parse generic type arguments
fn parse_generic_args(
    ty: &Type,
    type_name: &str,
    args: &syn::punctuated::Punctuated<GenericArgument, syn::token::Comma>,
) -> proc_macro2::TokenStream {
    match (type_name, args.len()) {
        ("Option" | "Vec" | "RecordLink", 1) => {
            if let Some(GenericArgument::Type(inner_ty)) = args.first() {
                let inner_parsed = parse_data_type(inner_ty);
                match type_name {
                    "Option" => {
                        quote! { ::helpers::evenframe::types::FieldType::Option(Box::new(#inner_parsed)) }
                    }
                    "Vec" => {
                        quote! { ::helpers::evenframe::types::FieldType::Vec(Box::new(#inner_parsed)) }
                    }
                    "RecordLink" => {
                        quote! { ::helpers::evenframe::types::FieldType::RecordLink(Box::new(#inner_parsed)) }
                    }
                    _ => unreachable!(),
                }
            } else {
                syn::Error::new(
                    args.span(),
                    format!("{} type parameter must be a type", type_name),
                )
                .to_compile_error()
            }
        }
        ("HashMap" | "BTreeMap", 2) => {
            let mut args_iter = args.iter();
            match (args_iter.next(), args_iter.next()) {
                (Some(GenericArgument::Type(key_ty)), Some(GenericArgument::Type(value_ty))) => {
                    let key_parsed = parse_data_type(key_ty);
                    let value_parsed = parse_data_type(value_ty);
                    match type_name {
                        "HashMap" => {
                            quote! { ::helpers::evenframe::types::FieldType::HashMap(Box::new(#key_parsed), Box::new(#value_parsed)) }
                        }
                        "BTreeMap" => {
                            quote! { ::helpers::evenframe::types::FieldType::BTreeMap(Box::new(#key_parsed), Box::new(#value_parsed)) }
                        }
                        _ => unreachable!(),
                    }
                }
                _ => syn::Error::new(
                    args.span(),
                    format!("{} type parameters must be types", type_name),
                )
                .to_compile_error(),
            }
        }
        ("DateTime" | "Duration", _) => {
            // These can have type params but we ignore them
            match type_name {
                "DateTime" => quote! { ::helpers::evenframe::types::FieldType::DateTime },
                "Duration" => quote! { ::helpers::evenframe::types::FieldType::Duration },
                _ => unreachable!(),
            }
        }
        (name, count) => {
            let expected = match name {
                "Option" | "Vec" | "RecordLink" => 1,
                "HashMap" | "BTreeMap" => 2,
                _ => {
                    return unsupported_type_error(
                        ty,
                        &format!("{}<...>", name),
                        "Unknown generic type",
                    )
                }
            };
            syn::Error::new(
                args.span(),
                format!(
                    "{} must have exactly {} type parameter{}, found {}",
                    name,
                    expected,
                    if expected == 1 { "" } else { "s" },
                    count
                ),
            )
            .to_compile_error()
        }
    }
}

/// Parse simple type by name
fn parse_simple_type(name: &str) -> Option<proc_macro2::TokenStream> {
    match name {
        "String" => Some(quote! { ::helpers::evenframe::types::FieldType::String }),
        "char" => Some(quote! { ::helpers::evenframe::types::FieldType::Char }),
        "bool" => Some(quote! { ::helpers::evenframe::types::FieldType::Bool }),
        "f32" => Some(quote! { ::helpers::evenframe::types::FieldType::F32 }),
        "f64" => Some(quote! { ::helpers::evenframe::types::FieldType::F64 }),
        "i8" => Some(quote! { ::helpers::evenframe::types::FieldType::I8 }),
        "i16" => Some(quote! { ::helpers::evenframe::types::FieldType::I16 }),
        "i32" => Some(quote! { ::helpers::evenframe::types::FieldType::I32 }),
        "i64" => Some(quote! { ::helpers::evenframe::types::FieldType::I64 }),
        "i128" => Some(quote! { ::helpers::evenframe::types::FieldType::I128 }),
        "isize" => Some(quote! { ::helpers::evenframe::types::FieldType::Isize }),
        "u8" => Some(quote! { ::helpers::evenframe::types::FieldType::U8 }),
        "u16" => Some(quote! { ::helpers::evenframe::types::FieldType::U16 }),
        "u32" => Some(quote! { ::helpers::evenframe::types::FieldType::U32 }),
        "u64" => Some(quote! { ::helpers::evenframe::types::FieldType::U64 }),
        "u128" => Some(quote! { ::helpers::evenframe::types::FieldType::U128 }),
        "usize" => Some(quote! { ::helpers::evenframe::types::FieldType::Usize }),
        "EvenframeRecordId" => {
            Some(quote! { ::helpers::evenframe::types::FieldType::EvenframeRecordId })
        }
        "Decimal" => Some(quote! { ::helpers::evenframe::types::FieldType::Decimal }),
        "DateTime" => Some(quote! { ::helpers::evenframe::types::FieldType::DateTime }),
        "Duration" => Some(quote! { ::helpers::evenframe::types::FieldType::Duration }),
        "Tz" => Some(quote! { ::helpers::evenframe::types::FieldType::Timezone }),
        "()" => Some(quote! { ::helpers::evenframe::types::FieldType::Unit }),
        _ => None,
    }
}

/// Check for common type mistakes
fn check_common_mistakes(ident: &syn::Ident) -> Option<proc_macro2::TokenStream> {
    match ident.to_string().as_str() {
        "str" => Some(syn::Error::new(
            ident.span(),
            "Use 'String' instead of 'str' for owned string types"
        ).to_compile_error()),
        "int" | "float" | "double" => Some(syn::Error::new(
            ident.span(),
            format!("'{}' is not a Rust type. Use i32/i64 for integers or f32/f64 for floating-point numbers", ident)
        ).to_compile_error()),
        _ => None,
    }
}

/// Parse a Rust type into its corresponding FieldType representation
pub fn parse_data_type(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        // Handle reference types
        Type::Reference(type_ref) => syn::Error::new(
            type_ref.span(),
            "Reference types (&T or &mut T) are not supported. \
            Use owned types instead (e.g., String instead of &str)",
        )
        .to_compile_error(),

        // Handle pointer types
        Type::Ptr(_) => syn::Error::new(
            ty.span(),
            "Raw pointer types are not supported in Evenframe schemas",
        )
        .to_compile_error(),

        // Handle array types
        Type::Array(arr) => syn::Error::new(
            arr.span(),
            "Fixed-size arrays are not supported. Use Vec<T> for dynamic arrays instead",
        )
        .to_compile_error(),

        // Handle slice types
        Type::Slice(slice) => syn::Error::new(
            slice.span(),
            "Slice types are not supported. Use Vec<T> instead",
        )
        .to_compile_error(),

        // Handle tuple types
        Type::Tuple(tuple) => {
            let elems = tuple.elems.iter().map(|elem| parse_data_type(elem));
            quote! { ::helpers::evenframe::types::FieldType::Tuple(vec![ #(#elems),* ]) }
        }

        // Handle path types (the most common case)
        Type::Path(type_path) => parse_path_type(ty, type_path),

        // Fallback for any other type
        _ => {
            let type_str = quote! { #ty }.to_string();
            unsupported_type_error(ty, &type_str, "This type pattern is not supported")
        }
    }
}

/// Parse path types (e.g., String, Vec<T>, std::collections::HashMap<K, V>)
fn parse_path_type(ty: &Type, type_path: &syn::TypePath) -> proc_macro2::TokenStream {
    let type_str = quote! { #ty }.to_string();

    // Get the last segment of the path (the actual type name)
    if let Some(last_segment) = type_path.path.segments.last() {
        let ident = &last_segment.ident;
        let ident_str = ident.to_string();

        // Check for common mistakes
        if let Some(error) = check_common_mistakes(ident) {
            return error;
        }

        // Check if it's a known simple type
        if let Some(field_type) = parse_simple_type(&ident_str) {
            return field_type;
        }

        // Check if it has generic arguments
        if let PathArguments::AngleBracketed(angle_args) = &last_segment.arguments {
            return parse_generic_args(ty, &ident_str, &angle_args.args);
        }

        // Handle known types without generic args that might be namespaced
        match ident_str.as_str() {
            "DateTime" => return quote! { ::helpers::evenframe::types::FieldType::DateTime },
            "Duration" => return quote! { ::helpers::evenframe::types::FieldType::Duration },
            "Decimal" => return quote! { ::helpers::evenframe::types::FieldType::Decimal },
            "Tz" => return quote! { ::helpers::evenframe::types::FieldType::Timezone },
            _ => {}
        }

        // Check for common namespaced types
        if type_str.ends_with("::String") {
            return quote! { ::helpers::evenframe::types::FieldType::String };
        }
        if type_str.ends_with("::HashMap") || type_str.ends_with("::BTreeMap") {
            return syn::Error::new(
                ty.span(),
                format!("'{}' requires type parameters. Use {}::<K, V> where K and V are your key and value types", type_str, type_str)
            ).to_compile_error();
        }
        if type_str.ends_with("::Vec") || type_str.ends_with("::Option") {
            return syn::Error::new(
                ty.span(),
                format!(
                    "'{}' requires a type parameter. Use {}::<T> where T is your inner type",
                    type_str, type_str
                ),
            )
            .to_compile_error();
        }
    }

    // If we get here, it's a custom type (struct/enum)
    let lit = syn::LitStr::new(&type_str, ty.span());
    quote! { ::helpers::evenframe::types::FieldType::Other(#lit.to_string()) }
}
