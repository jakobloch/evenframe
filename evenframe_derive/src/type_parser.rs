use quote::quote;
use syn::{GenericArgument, PathArguments, Type};
use syn::spanned::Spanned;

pub fn parse_data_type(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        // Handle simple paths (e.g. "String", "bool", etc.)
        Type::Path(type_path) => {
            // If there's a single segment, we check for known identifiers.
            if type_path.qself.is_none() && type_path.path.segments.len() == 1 {
                let ident = &type_path.path.segments.first().unwrap().ident;
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
                    "DateTime" => quote! { ::helpers::evenframe::schemasync::FieldType::DateTime },
                    "Duration" => quote! { ::helpers::evenframe::schemasync::FieldType::Duration },
                    "Tz" => quote! { ::helpers::evenframe::schemasync::FieldType::Timezone },
                    "()" => quote! { ::helpers::evenframe::schemasync::FieldType::Unit },
                    _ => {
                        // Check if this is a path with generic arguments
                        let args = &type_path.path.segments.first().unwrap().arguments;
                        if let PathArguments::AngleBracketed(angle_args) = args {
                            let ident_str = ident.to_string();

                            if ident_str == "Option" && angle_args.args.len() == 1 {
                                if let Some(GenericArgument::Type(inner_ty)) =
                                    angle_args.args.first()
                                {
                                    let inner_parsed = parse_data_type(inner_ty);
                                    return quote! { ::helpers::evenframe::schemasync::FieldType::Option(Box::new(#inner_parsed)) };
                                }
                            } else if ident_str == "Vec" && angle_args.args.len() == 1 {
                                if let Some(GenericArgument::Type(inner_ty)) =
                                    angle_args.args.first()
                                {
                                    let inner_parsed = parse_data_type(inner_ty);
                                    return quote! { ::helpers::evenframe::schemasync::FieldType::Vec(Box::new(#inner_parsed)) };
                                }
                            } else if ident_str == "HashMap" && angle_args.args.len() == 2 {
                                let mut args_iter = angle_args.args.iter();
                                if let (
                                    Some(GenericArgument::Type(key_ty)),
                                    Some(GenericArgument::Type(value_ty)),
                                ) = (args_iter.next(), args_iter.next())
                                {
                                    let key_parsed = parse_data_type(key_ty);
                                    let value_parsed = parse_data_type(value_ty);
                                    return quote! { ::helpers::evenframe::schemasync::FieldType::HashMap(Box::new(#key_parsed), Box::new(#value_parsed)) };
                                }
                            } else if ident_str == "BTreeMap" && angle_args.args.len() == 2 {
                                let mut args_iter = angle_args.args.iter();
                                if let (
                                    Some(GenericArgument::Type(key_ty)),
                                    Some(GenericArgument::Type(value_ty)),
                                ) = (args_iter.next(), args_iter.next())
                                {
                                    let key_parsed = parse_data_type(key_ty);
                                    let value_parsed = parse_data_type(value_ty);
                                    return quote! { ::helpers::evenframe::schemasync::FieldType::BTreeMap(Box::new(#key_parsed), Box::new(#value_parsed)) };
                                }
                            } else if ident_str == "RecordLink" && angle_args.args.len() == 1 {
                                if let Some(GenericArgument::Type(inner_ty)) =
                                    angle_args.args.first()
                                {
                                    let inner_parsed = parse_data_type(inner_ty);
                                    return quote! { ::helpers::evenframe::schemasync::FieldType::RecordLink(Box::new(#inner_parsed)) };
                                }
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
                                        let lit = syn::LitStr::new(&type_str, ty.span());
                                        quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                                    }
                                } else if outer == "Vec" {
                                    if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                                        let inner_parsed = parse_data_type(&inner_ty);
                                        quote! { ::helpers::evenframe::schemasync::FieldType::Vec(Box::new(#inner_parsed)) }
                                    } else {
                                        let lit = syn::LitStr::new(&type_str, ty.span());
                                        quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
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
                                            let lit = syn::LitStr::new(&type_str, ty.span());
                                            quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                                        }
                                    } else {
                                        let lit = syn::LitStr::new(&type_str, ty.span());
                                        quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
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
                                            let lit = syn::LitStr::new(&type_str, ty.span());
                                            quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                                        }
                                    } else {
                                        let lit = syn::LitStr::new(&type_str, ty.span());
                                        quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                                    }
                                } else if outer == "RecordLink" {
                                    // Parse RecordLink<T>
                                    if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                                        let inner_parsed = parse_data_type(&inner_ty);
                                        quote! { ::helpers::evenframe::schemasync::FieldType::RecordLink(Box::new(#inner_parsed)) }
                                    } else {
                                        let lit = syn::LitStr::new(&type_str, ty.span());
                                        quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                                    }
                                } else if outer == "DateTime" {
                                    // Handle DateTime<Utc> and similar types
                                    quote! { ::helpers::evenframe::schemasync::FieldType::DateTime }
                                } else if outer == "Duration" {
                                    // Handle Duration and similar types
                                    quote! { ::helpers::evenframe::schemasync::FieldType::Duration }
                                } else {
                                    let lit = syn::LitStr::new(&type_str, ty.span());
                                    quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                                }
                            } else {
                                let lit = syn::LitStr::new(&type_str, ty.span());
                                quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
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

                let lit = syn::LitStr::new(&type_str, ty.span());
                quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
            }
        }
        // For tuple types, recursively convert each element.
        Type::Tuple(tuple) => {
            let elems = tuple.elems.iter().map(|elem| parse_data_type(elem));
            quote! { ::helpers::evenframe::schemasync::FieldType::Tuple(vec![ #(#elems),* ]) }
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
                            let lit = syn::LitStr::new(&type_str, ty.span());
                            quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                        }
                    } else if outer == "Vec" {
                        if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                            let inner_parsed = parse_data_type(&inner_ty);
                            quote! { ::helpers::evenframe::schemasync::FieldType::Vec(Box::new(#inner_parsed)) }
                        } else {
                            let lit = syn::LitStr::new(&type_str, ty.span());
                            quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
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
                                let lit = syn::LitStr::new(&type_str, ty.span());
                                quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                            }
                        } else {
                            let lit = syn::LitStr::new(&type_str, ty.span());
                            quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
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
                                let lit = syn::LitStr::new(&type_str, ty.span());
                                quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                            }
                        } else {
                            let lit = syn::LitStr::new(&type_str, ty.span());
                            quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                        }
                    } else if outer == "RecordLink" {
                        // Parse RecordLink<T>
                        if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                            let inner_parsed = parse_data_type(&inner_ty);
                            quote! { ::helpers::evenframe::schemasync::FieldType::RecordLink(Box::new(#inner_parsed)) }
                        } else {
                            let lit = syn::LitStr::new(&type_str, ty.span());
                            quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                        }
                    } else if outer == "DateTime" || outer.ends_with("DateTime") {
                        // Handle DateTime<Utc> and similar types, including chrono::DateTime
                        quote! { ::helpers::evenframe::schemasync::FieldType::DateTime }
                    } else if outer == "Duration" || outer.ends_with("Duration") {
                        // Handle Duration and similar types, including chrono::Duration
                        quote! { ::helpers::evenframe::schemasync::FieldType::Duration }
                    } else {
                        let lit = syn::LitStr::new(&type_str, ty.span());
                        quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                    }
                } else {
                    let lit = syn::LitStr::new(&type_str, ty.span());
                    quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                }
            } else {
                let lit = syn::LitStr::new(&type_str, ty.span());
                quote! { ::helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
            }
        }
    }
}