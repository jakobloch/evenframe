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
        )
    ).to_compile_error()
}

/// Parse a Rust type into its corresponding FieldType representation
/// 
/// # Errors
/// 
/// This function generates compile-time errors for:
/// - Unsupported generic types
/// - Malformed type syntax
/// - Invalid type combinations
pub fn parse_data_type(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        // Handle simple paths (e.g. "String", "bool", etc.)
        Type::Path(type_path) => {
            // If there's a single segment, we check for known identifiers.
            if type_path.qself.is_none() && type_path.path.segments.len() == 1 {
                let segment = type_path.path.segments.first()
                    .expect("Type path should have at least one segment");
                let ident = &segment.ident;
                match ident.to_string().as_str() {
                    "String" => quote! { ::helpers::evenframe::schemasync::FieldType::String },
                    "char" => quote! { ::helpers::evenframe::schemasync::FieldType::Char },
                    "bool" => quote! { ::helpers::evenframe::schemasync::FieldType::Bool },
                    "f32" => quote! { ::helpers::evenframe::schemasync::FieldType::F32 },
                    "f64" => quote! { ::helpers::evenframe::schemasync::FieldType::F64 },
                    "i8" => quote! { ::helpers::evenframe::schemasync::FieldType::I8 },
                    "i16" => quote! { ::helpers::evenframe::schemasync::FieldType::I16 },
                    "i32" => quote! { ::helpers::evenframe::schemasync::FieldType::I32 },
                    "i64" => quote! { ::helpers::evenframe::schemasync::FieldType::I64 },
                    "i128" => quote! { ::helpers::evenframe::schemasync::FieldType::I128 },
                    "isize" => quote! { ::helpers::evenframe::schemasync::FieldType::Isize },
                    "u8" => quote! { ::helpers::evenframe::schemasync::FieldType::U8 },
                    "u16" => quote! { ::helpers::evenframe::schemasync::FieldType::U16 },
                    "u32" => quote! { ::helpers::evenframe::schemasync::FieldType::U32 },
                    "u64" => quote! { ::helpers::evenframe::schemasync::FieldType::U64 },
                    "u128" => quote! { ::helpers::evenframe::schemasync::FieldType::U128 },
                    "usize" => quote! { ::helpers::evenframe::schemasync::FieldType::Usize },
                    "EvenframeRecordId" => {
                        quote! { ::helpers::evenframe::schemasync::FieldType::EvenframeRecordId }
                    }
                    "Decimal" => quote! { ::helpers::evenframe::schemasync::FieldType::Decimal },
                    "DateTime" => quote! { ::helpers::evenframe::schemasync::FieldType::DateTime },
                    "Duration" => quote! { ::helpers::evenframe::schemasync::FieldType::Duration },
                    "Tz" => quote! { ::helpers::evenframe::schemasync::FieldType::Timezone },
                    "()" => quote! { ::helpers::evenframe::schemasync::FieldType::Unit },
                    // Common mistakes
                    "str" => {
                        return syn::Error::new(
                            ident.span(),
                            "Use 'String' instead of 'str' for owned string types"
                        ).to_compile_error();
                    }
                    "int" | "float" | "double" => {
                        return syn::Error::new(
                            ident.span(),
                            format!("'{}' is not a Rust type. Use i32/i64 for integers or f32/f64 for floating-point numbers", ident)
                        ).to_compile_error();
                    }
                    _ => {
                        // Check if this is a path with generic arguments
                        let args = &segment.arguments;
                        if let PathArguments::AngleBracketed(angle_args) = args {
                            let ident_str = ident.to_string();

                            if ident_str == "Option" {
                                if angle_args.args.len() != 1 {
                                    return syn::Error::new(
                                        angle_args.span(),
                                        format!("Option type must have exactly one type parameter, found {}", angle_args.args.len())
                                    ).to_compile_error();
                                }
                                if let Some(GenericArgument::Type(inner_ty)) =
                                    angle_args.args.first()
                                {
                                    let inner_parsed = parse_data_type(inner_ty);
                                    return quote! { ::helpers::evenframe::schemasync::FieldType::Option(Box::new(#inner_parsed)) };
                                }
                                return syn::Error::new(
                                    angle_args.span(),
                                    "Option type parameter must be a type"
                                ).to_compile_error();
                            } else if ident_str == "Vec" {
                                if angle_args.args.len() != 1 {
                                    return syn::Error::new(
                                        angle_args.span(),
                                        format!("Vec type must have exactly one type parameter, found {}", angle_args.args.len())
                                    ).to_compile_error();
                                }
                                if let Some(GenericArgument::Type(inner_ty)) =
                                    angle_args.args.first()
                                {
                                    let inner_parsed = parse_data_type(inner_ty);
                                    return quote! { ::helpers::evenframe::schemasync::FieldType::Vec(Box::new(#inner_parsed)) };
                                }
                                return syn::Error::new(
                                    angle_args.span(),
                                    "Vec type parameter must be a type"
                                ).to_compile_error();
                            } else if ident_str == "HashMap" {
                                if angle_args.args.len() != 2 {
                                    return syn::Error::new(
                                        angle_args.span(),
                                        format!("HashMap must have exactly two type parameters (key and value), found {}", angle_args.args.len())
                                    ).to_compile_error();
                                }
                                let mut args_iter = angle_args.args.iter();
                                match (args_iter.next(), args_iter.next()) {
                                    (Some(GenericArgument::Type(key_ty)), Some(GenericArgument::Type(value_ty))) => {
                                        let key_parsed = parse_data_type(key_ty);
                                        let value_parsed = parse_data_type(value_ty);
                                        return quote! { ::helpers::evenframe::schemasync::FieldType::HashMap(Box::new(#key_parsed), Box::new(#value_parsed)) };
                                    }
                                    _ => {
                                        return syn::Error::new(
                                            angle_args.span(),
                                            "HashMap type parameters must be types"
                                        ).to_compile_error();
                                    }
                                }
                            } else if ident_str == "BTreeMap" {
                                if angle_args.args.len() != 2 {
                                    return syn::Error::new(
                                        angle_args.span(),
                                        format!("BTreeMap must have exactly two type parameters (key and value), found {}", angle_args.args.len())
                                    ).to_compile_error();
                                }
                                let mut args_iter = angle_args.args.iter();
                                match (args_iter.next(), args_iter.next()) {
                                    (Some(GenericArgument::Type(key_ty)), Some(GenericArgument::Type(value_ty))) => {
                                        let key_parsed = parse_data_type(key_ty);
                                        let value_parsed = parse_data_type(value_ty);
                                        return quote! { ::helpers::evenframe::schemasync::FieldType::BTreeMap(Box::new(#key_parsed), Box::new(#value_parsed)) };
                                    }
                                    _ => {
                                        return syn::Error::new(
                                            angle_args.span(),
                                            "BTreeMap type parameters must be types"
                                        ).to_compile_error();
                                    }
                                }
                            } else if ident_str == "RecordLink" {
                                if angle_args.args.len() != 1 {
                                    return syn::Error::new(
                                        angle_args.span(),
                                        format!("RecordLink must have exactly one type parameter, found {}", angle_args.args.len())
                                    ).to_compile_error();
                                }
                                if let Some(GenericArgument::Type(inner_ty)) =
                                    angle_args.args.first()
                                {
                                    let inner_parsed = parse_data_type(inner_ty);
                                    return quote! { ::helpers::evenframe::schemasync::FieldType::RecordLink(Box::new(#inner_parsed)) };
                                }
                                return syn::Error::new(
                                    angle_args.span(),
                                    "RecordLink type parameter must be a type"
                                ).to_compile_error();
                            } else if ident_str == "DateTime" {
                                // Handle DateTime<Utc> and similar types
                                return quote! { ::helpers::evenframe::schemasync::FieldType::DateTime };
                            } else if ident_str == "Duration" {
                                // Handle Duration and similar types
                                return quote! { ::helpers::evenframe::schemasync::FieldType::Duration };
                            }
                        }

                        // Convert the type into a string and remove all whitespace.
                        let type_str: String = quote! { #ty }
                            .to_string()
                            .chars()
                            .filter(|c| !c.is_whitespace())
                            .collect();

                        // Look for the generic delimiters.
                        if let Some(start) = type_str.find('<') {
                            if let Some(end) = type_str.rfind('>') {
                                let outer = &type_str[..start];
                                let inner = &type_str[start + 1..end];

                                if outer == "Option" {
                                    if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                                        let inner_parsed = parse_data_type(&inner_ty);
                                        quote! { ::helpers::evenframe::schemasync::FieldType::Option(Box::new(#inner_parsed)) }
                                    } else {
                                        return syn::Error::new(
                                            ty.span(),
                                            format!("Failed to parse inner type of Option: '{}'. Consider using a standard type or check for typos.", inner)
                                        ).to_compile_error();
                                    }
                                } else if outer == "Vec" {
                                    if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                                        let inner_parsed = parse_data_type(&inner_ty);
                                        quote! { ::helpers::evenframe::schemasync::FieldType::Vec(Box::new(#inner_parsed)) }
                                    } else {
                                        return syn::Error::new(
                                            ty.span(),
                                            format!("Failed to parse inner type of Vec: '{}'. Consider using a standard type or check for typos.", inner)
                                        ).to_compile_error();
                                    }
                                } else if outer == "HashMap" {
                                    // Parse HashMap<K, V>
                                    if let Some(comma_idx) = inner.find(',') {
                                        let key_str = inner[..comma_idx].trim();
                                        let value_str = inner[comma_idx + 1..].trim();

                                        if let (Ok(key_ty), Ok(value_ty)) = (
                                            syn::parse_str::<Type>(key_str),
                                            syn::parse_str::<Type>(value_str),
                                        ) {
                                            let key_parsed = parse_data_type(&key_ty);
                                            let value_parsed = parse_data_type(&value_ty);
                                            quote! { ::helpers::evenframe::schemasync::FieldType::HashMap(Box::new(#key_parsed), Box::new(#value_parsed)) }
                                        } else {
                                            return syn::Error::new(
                                                ty.span(),
                                                format!("Failed to parse HashMap type parameters: key='{}', value='{}'. Both must be valid types.", key_str, value_str)
                                            ).to_compile_error();
                                        }
                                    } else {
                                        return syn::Error::new(
                                            ty.span(),
                                            "HashMap requires two type parameters separated by a comma (e.g., HashMap<String, i32>)"
                                        ).to_compile_error();
                                    }
                                } else if outer == "BTreeMap" {
                                    // Parse BTreeMap<K, V>
                                    if let Some(comma_idx) = inner.find(',') {
                                        let key_str = inner[..comma_idx].trim();
                                        let value_str = inner[comma_idx + 1..].trim();

                                        if let (Ok(key_ty), Ok(value_ty)) = (
                                            syn::parse_str::<Type>(key_str),
                                            syn::parse_str::<Type>(value_str),
                                        ) {
                                            let key_parsed = parse_data_type(&key_ty);
                                            let value_parsed = parse_data_type(&value_ty);
                                            quote! { ::helpers::evenframe::schemasync::FieldType::BTreeMap(Box::new(#key_parsed), Box::new(#value_parsed)) }
                                        } else {
                                            return syn::Error::new(
                                                ty.span(),
                                                format!("Failed to parse BTreeMap type parameters: key='{}', value='{}'. Both must be valid types.", key_str, value_str)
                                            ).to_compile_error();
                                        }
                                    } else {
                                        return syn::Error::new(
                                            ty.span(),
                                            "BTreeMap requires two type parameters separated by a comma (e.g., BTreeMap<String, i32>)"
                                        ).to_compile_error();
                                    }
                                } else if outer == "RecordLink" {
                                    // Parse RecordLink<T>
                                    if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                                        let inner_parsed = parse_data_type(&inner_ty);
                                        quote! { ::helpers::evenframe::schemasync::FieldType::RecordLink(Box::new(#inner_parsed)) }
                                    } else {
                                        return syn::Error::new(
                                            ty.span(),
                                            format!("Failed to parse inner type of RecordLink: '{}'. Consider using a standard type or check for typos.", inner)
                                        ).to_compile_error();
                                    }
                                } else if outer == "DateTime" {
                                    // Handle DateTime<Utc> and similar types
                                    quote! { ::helpers::evenframe::schemasync::FieldType::DateTime }
                                } else if outer == "Duration" {
                                    // Handle Duration and similar types
                                    quote! { ::helpers::evenframe::schemasync::FieldType::Duration }
                                } else {
                                    // Unknown generic type
                                    return unsupported_type_error(ty, &type_str, 
                                        "This appears to be a generic type that is not recognized.");
                                }
                            } else {
                                // Malformed generic syntax
                                return syn::Error::new(
                                    ty.span(),
                                    format!("Malformed generic type syntax in '{}'. Generic types should have matching < and > brackets.", type_str)
                                ).to_compile_error();
                            }
                        } else {
                            let lit = syn::LitStr::new(&type_str, ty.span());
                            quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                        }
                    }
                }
            } else {
                // For complex type paths, check if it's DateTime
                let type_str = quote! { #ty }.to_string();

                // Check if this is a DateTime type (e.g., chrono::DateTime<Utc>)
                if type_path
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident == "DateTime")
                    .unwrap_or(false)
                {
                    return quote! { ::helpers::evenframe::schemasync::FieldType::DateTime };
                }
                // Check if this is a Duration type (e.g., chrono::Duration)
                if type_path
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident == "Duration")
                    .unwrap_or(false)
                {
                    return quote! { ::helpers::evenframe::schemasync::FieldType::Duration };
                }
                // Check if this is a Tz type (e.g., chrono_tz::Tz)
                if type_path
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident == "Tz")
                    .unwrap_or(false)
                {
                    return quote! { ::helpers::evenframe::schemasync::FieldType::Timezone };
                }
                // Check if this is a Decimal type (e.g., rust_decimal::Decimal)
                if type_path
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident == "Decimal")
                    .unwrap_or(false)
                {
                    return quote! { ::helpers::evenframe::schemasync::FieldType::Decimal };
                }

                // Check for common std types that might be namespaced
                if type_str.ends_with("::String") {
                    return quote! { ::helpers::evenframe::schemasync::FieldType::String };
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
                        format!("'{}' requires a type parameter. Use {}::<T> where T is your inner type", type_str, type_str)
                    ).to_compile_error();
                }

                let lit = syn::LitStr::new(&type_str, ty.span());
                quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
            }
        }
        // For tuple types, recursively convert each element.
        Type::Tuple(tuple) => {
            let elems = tuple.elems.iter().map(|elem| parse_data_type(elem));
            quote! { ::helpers::evenframe::schemasync::FieldType::Tuple(vec![ #(#elems),* ]) }
        }
        // Handle reference types
        Type::Reference(type_ref) => {
            return syn::Error::new(
                type_ref.span(),
                "Reference types (&T or &mut T) are not supported. \
                Use owned types instead (e.g., String instead of &str)"
            ).to_compile_error();
        }
        // Handle pointer types
        Type::Ptr(_) => {
            return syn::Error::new(
                ty.span(),
                "Raw pointer types are not supported in Evenframe schemas"
            ).to_compile_error();
        }
        // Handle array types
        Type::Array(arr) => {
            return syn::Error::new(
                arr.span(),
                "Fixed-size arrays are not supported. Use Vec<T> for dynamic arrays instead"
            ).to_compile_error();
        }
        // Handle slice types
        Type::Slice(slice) => {
            return syn::Error::new(
                slice.span(),
                "Slice types are not supported. Use Vec<T> instead"
            ).to_compile_error();
        }
        // Fallback for any other type.
        _ => {
            // Convert the type into a string and remove all whitespace.
            let type_str: String = quote! { #ty }
                .to_string()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();

            // Look for the generic delimiters.
            if let Some(start) = type_str.find('<') {
                if let Some(end) = type_str.rfind('>') {
                    let outer = &type_str[..start];
                    let inner = &type_str[start + 1..end];

                    if outer == "Option" {
                        if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                            let inner_parsed = parse_data_type(&inner_ty);
                            quote! { ::helpers::evenframe::schemasync::FieldType::Option(Box::new(#inner_parsed)) }
                        } else {
                            return syn::Error::new(
                                ty.span(),
                                format!("Failed to parse inner type of Option: '{}'. Consider using a standard type or check for typos.", inner)
                            ).to_compile_error();
                        }
                    } else if outer == "Vec" {
                        if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                            let inner_parsed = parse_data_type(&inner_ty);
                            quote! { ::helpers::evenframe::schemasync::FieldType::Vec(Box::new(#inner_parsed)) }
                        } else {
                            return syn::Error::new(
                                ty.span(),
                                format!("Failed to parse inner type of Vec: '{}'. Consider using a standard type or check for typos.", inner)
                            ).to_compile_error();
                        }
                    } else if outer == "HashMap" {
                        // Parse HashMap<K, V>
                        if let Some(comma_idx) = inner.find(',') {
                            let key_str = inner[..comma_idx].trim();
                            let value_str = inner[comma_idx + 1..].trim();

                            if let (Ok(key_ty), Ok(value_ty)) = (
                                syn::parse_str::<Type>(key_str),
                                syn::parse_str::<Type>(value_str),
                            ) {
                                let key_parsed = parse_data_type(&key_ty);
                                let value_parsed = parse_data_type(&value_ty);
                                quote! { ::helpers::evenframe::schemasync::FieldType::HashMap(Box::new(#key_parsed), Box::new(#value_parsed)) }
                            } else {
                                return syn::Error::new(
                                    ty.span(),
                                    format!("Failed to parse HashMap type parameters: key='{}', value='{}'. Both must be valid types.", key_str, value_str)
                                ).to_compile_error();
                            }
                        } else {
                            return syn::Error::new(
                                ty.span(),
                                "HashMap requires two type parameters separated by a comma (e.g., HashMap<String, i32>)"
                            ).to_compile_error();
                        }
                    } else if outer == "BTreeMap" {
                        // Parse BTreeMap<K, V>
                        if let Some(comma_idx) = inner.find(',') {
                            let key_str = inner[..comma_idx].trim();
                            let value_str = inner[comma_idx + 1..].trim();

                            if let (Ok(key_ty), Ok(value_ty)) = (
                                syn::parse_str::<Type>(key_str),
                                syn::parse_str::<Type>(value_str),
                            ) {
                                let key_parsed = parse_data_type(&key_ty);
                                let value_parsed = parse_data_type(&value_ty);
                                quote! { ::helpers::evenframe::schemasync::FieldType::BTreeMap(Box::new(#key_parsed), Box::new(#value_parsed)) }
                            } else {
                                return syn::Error::new(
                                    ty.span(),
                                    format!("Failed to parse BTreeMap type parameters: key='{}', value='{}'. Both must be valid types.", key_str, value_str)
                                ).to_compile_error();
                            }
                        } else {
                            return syn::Error::new(
                                ty.span(),
                                "BTreeMap requires two type parameters separated by a comma (e.g., BTreeMap<String, i32>)"
                            ).to_compile_error();
                        }
                    } else if outer == "RecordLink" {
                        // Parse RecordLink<T>
                        if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                            let inner_parsed = parse_data_type(&inner_ty);
                            quote! { ::helpers::evenframe::schemasync::FieldType::RecordLink(Box::new(#inner_parsed)) }
                        } else {
                            return syn::Error::new(
                                ty.span(),
                                format!("Failed to parse inner type of RecordLink: '{}'. Consider using a standard type or check for typos.", inner)
                            ).to_compile_error();
                        }
                    } else if outer == "DateTime" || outer.ends_with("DateTime") {
                        // Handle DateTime<Utc> and similar types, including chrono::DateTime
                        quote! { ::helpers::evenframe::schemasync::FieldType::DateTime }
                    } else if outer == "Duration" || outer.ends_with("Duration") {
                        // Handle Duration and similar types, including chrono::Duration
                        quote! { ::helpers::evenframe::schemasync::FieldType::Duration }
                    } else {
                        // Unknown generic type in fallback
                        return unsupported_type_error(ty, &type_str, 
                            "This appears to be a generic type that is not recognized.");
                    }
                } else {
                    // Malformed generic syntax
                    return syn::Error::new(
                        ty.span(),
                        format!("Malformed generic type syntax in '{}'. Generic types should have matching < and > brackets.", type_str)
                    ).to_compile_error();
                }
            } else {
                // Non-generic type that wasn't recognized
                let lit = syn::LitStr::new(&type_str, ty.span());
                quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
            }
        }
    }
}
