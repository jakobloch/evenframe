mod coordinate_parsing;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, Data, DeriveInput, Expr, ExprLit, Fields,
    GenericArgument, Lit, LitStr, Meta, PathArguments, Type,
};

fn parse_data_type(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        // Handle simple paths (e.g. "String", "bool", etc.)
        Type::Path(type_path) => {
            // If there's a single segment, we check for known identifiers.
            if type_path.qself.is_none() && type_path.path.segments.len() == 1 {
                let ident = &type_path.path.segments.first().unwrap().ident;
                match ident.to_string().as_str() {
                    "String" => quote! { helpers::evenframe::schemasync::FieldType::String },
                    "char" => quote! { helpers::evenframe::schemasync::FieldType::Char },
                    "bool" => quote! { helpers::evenframe::schemasync::FieldType::Bool },
                    "f32" => quote! { helpers::evenframe::schemasync::FieldType::F32 },
                    "f64" => quote! { helpers::evenframe::schemasync::FieldType::F64 },
                    "i8" => quote! { helpers::evenframe::schemasync::FieldType::I8 },
                    "i16" => quote! { helpers::evenframe::schemasync::FieldType::I16 },
                    "i32" => quote! { helpers::evenframe::schemasync::FieldType::I32 },
                    "i64" => quote! { helpers::evenframe::schemasync::FieldType::I64 },
                    "i128" => quote! { helpers::evenframe::schemasync::FieldType::I128 },
                    "isize" => quote! { helpers::evenframe::schemasync::FieldType::Isize },
                    "u8" => quote! { helpers::evenframe::schemasync::FieldType::U8 },
                    "u16" => quote! { helpers::evenframe::schemasync::FieldType::U16 },
                    "u32" => quote! { helpers::evenframe::schemasync::FieldType::U32 },
                    "u64" => quote! { helpers::evenframe::schemasync::FieldType::U64 },
                    "u128" => quote! { helpers::evenframe::schemasync::FieldType::U128 },
                    "usize" => quote! { helpers::evenframe::schemasync::FieldType::Usize },
                    "EvenframeRecordId" => {
                        quote! { helpers::evenframe::schemasync::FieldType::EvenframeRecordId }
                    }
                    "DateTime" => quote! { helpers::evenframe::schemasync::FieldType::DateTime },
                    "Duration" => quote! { helpers::evenframe::schemasync::FieldType::Duration },
                    "Tz" => quote! { helpers::evenframe::schemasync::FieldType::Timezone },
                    "()" => quote! { helpers::evenframe::schemasync::FieldType::Unit },
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
                                    return quote! { helpers::evenframe::schemasync::FieldType::Option(Box::new(#inner_parsed)) };
                                }
                            } else if ident_str == "Vec" && angle_args.args.len() == 1 {
                                if let Some(GenericArgument::Type(inner_ty)) =
                                    angle_args.args.first()
                                {
                                    let inner_parsed = parse_data_type(inner_ty);
                                    return quote! { helpers::evenframe::schemasync::FieldType::Vec(Box::new(#inner_parsed)) };
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
                                    return quote! { helpers::evenframe::schemasync::FieldType::HashMap(Box::new(#key_parsed), Box::new(#value_parsed)) };
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
                                    return quote! { helpers::evenframe::schemasync::FieldType::BTreeMap(Box::new(#key_parsed), Box::new(#value_parsed)) };
                                }
                            } else if ident_str == "RecordLink" && angle_args.args.len() == 1 {
                                if let Some(GenericArgument::Type(inner_ty)) =
                                    angle_args.args.first()
                                {
                                    let inner_parsed = parse_data_type(inner_ty);
                                    return quote! { helpers::evenframe::schemasync::FieldType::RecordLink(Box::new(#inner_parsed)) };
                                }
                            } else if ident_str == "DateTime" {
                                // Handle DateTime<Utc> and similar types
                                return quote! { helpers::evenframe::schemasync::FieldType::DateTime };
                            } else if ident_str == "Duration" {
                                // Handle Duration and similar types
                                return quote! { helpers::evenframe::schemasync::FieldType::Duration };
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
                                        quote! { helpers::evenframe::schemasync::FieldType::Option(Box::new(#inner_parsed)) }
                                    } else {
                                        let lit = syn::LitStr::new(&type_str, ty.span());
                                        quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                                    }
                                } else if outer == "Vec" {
                                    if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                                        let inner_parsed = parse_data_type(&inner_ty);
                                        quote! { helpers::evenframe::schemasync::FieldType::Vec(Box::new(#inner_parsed)) }
                                    } else {
                                        let lit = syn::LitStr::new(&type_str, ty.span());
                                        quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
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
                                            quote! { helpers::evenframe::schemasync::FieldType::HashMap(Box::new(#key_parsed), Box::new(#value_parsed)) }
                                        } else {
                                            let lit = syn::LitStr::new(&type_str, ty.span());
                                            quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                                        }
                                    } else {
                                        let lit = syn::LitStr::new(&type_str, ty.span());
                                        quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
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
                                            quote! { helpers::evenframe::schemasync::FieldType::BTreeMap(Box::new(#key_parsed), Box::new(#value_parsed)) }
                                        } else {
                                            let lit = syn::LitStr::new(&type_str, ty.span());
                                            quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                                        }
                                    } else {
                                        let lit = syn::LitStr::new(&type_str, ty.span());
                                        quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                                    }
                                } else if outer == "RecordLink" {
                                    // Parse RecordLink<T>
                                    if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                                        let inner_parsed = parse_data_type(&inner_ty);
                                        quote! { helpers::evenframe::schemasync::FieldType::RecordLink(Box::new(#inner_parsed)) }
                                    } else {
                                        let lit = syn::LitStr::new(&type_str, ty.span());
                                        quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                                    }
                                } else if outer == "DateTime" {
                                    // Handle DateTime<Utc> and similar types
                                    quote! { helpers::evenframe::schemasync::FieldType::DateTime }
                                } else if outer == "Duration" {
                                    // Handle Duration and similar types
                                    quote! { helpers::evenframe::schemasync::FieldType::Duration }
                                } else {
                                    let lit = syn::LitStr::new(&type_str, ty.span());
                                    quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                                }
                            } else {
                                let lit = syn::LitStr::new(&type_str, ty.span());
                                quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                            }
                        } else {
                            let lit = syn::LitStr::new(&type_str, ty.span());
                            quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
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
                    return quote! { helpers::evenframe::schemasync::FieldType::DateTime };
                }
                // Check if this is a Duration type (e.g., chrono::Duration)
                if type_path
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident == "Duration")
                    .unwrap_or(false)
                {
                    return quote! { helpers::evenframe::schemasync::FieldType::Duration };
                }
                // Check if this is a Tz type (e.g., chrono_tz::Tz)
                if type_path
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident == "Tz")
                    .unwrap_or(false)
                {
                    return quote! { helpers::evenframe::schemasync::FieldType::Timezone };
                }

                let lit = syn::LitStr::new(&type_str, ty.span());
                quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
            }
        }
        // For tuple types, recursively convert each element.
        Type::Tuple(tuple) => {
            let elems = tuple.elems.iter().map(|elem| parse_data_type(elem));
            quote! { helpers::evenframe::schemasync::FieldType::Tuple(vec![ #(#elems),* ]) }
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
                            quote! { helpers::evenframe::schemasync::FieldType::Option(Box::new(#inner_parsed)) }
                        } else {
                            let lit = syn::LitStr::new(&type_str, ty.span());
                            quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                        }
                    } else if outer == "Vec" {
                        if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                            let inner_parsed = parse_data_type(&inner_ty);
                            quote! { helpers::evenframe::schemasync::FieldType::Vec(Box::new(#inner_parsed)) }
                        } else {
                            let lit = syn::LitStr::new(&type_str, ty.span());
                            quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
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
                                quote! { helpers::evenframe::schemasync::FieldType::HashMap(Box::new(#key_parsed), Box::new(#value_parsed)) }
                            } else {
                                let lit = syn::LitStr::new(&type_str, ty.span());
                                quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                            }
                        } else {
                            let lit = syn::LitStr::new(&type_str, ty.span());
                            quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
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
                                quote! { helpers::evenframe::schemasync::FieldType::BTreeMap(Box::new(#key_parsed), Box::new(#value_parsed)) }
                            } else {
                                let lit = syn::LitStr::new(&type_str, ty.span());
                                quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                            }
                        } else {
                            let lit = syn::LitStr::new(&type_str, ty.span());
                            quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                        }
                    } else if outer == "RecordLink" {
                        // Parse RecordLink<T>
                        if let Ok(inner_ty) = syn::parse_str::<Type>(inner) {
                            let inner_parsed = parse_data_type(&inner_ty);
                            quote! { helpers::evenframe::schemasync::FieldType::RecordLink(Box::new(#inner_parsed)) }
                        } else {
                            let lit = syn::LitStr::new(&type_str, ty.span());
                            quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                        }
                    } else if outer == "DateTime" || outer.ends_with("DateTime") {
                        // Handle DateTime<Utc> and similar types, including chrono::DateTime
                        quote! { helpers::evenframe::schemasync::FieldType::DateTime }
                    } else if outer == "Duration" || outer.ends_with("Duration") {
                        // Handle Duration and similar types, including chrono::Duration
                        quote! { helpers::evenframe::schemasync::FieldType::Duration }
                    } else {
                        let lit = syn::LitStr::new(&type_str, ty.span());
                        quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                    }
                } else {
                    let lit = syn::LitStr::new(&type_str, ty.span());
                    quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
                }
            } else {
                let lit = syn::LitStr::new(&type_str, ty.span());
                quote! { helpers::evenframe::schemasync::FieldType::Other(#lit.to_string()) }
            }
        }
    }
}

fn parse_mock_data_attribute(
    attrs: &[Attribute],
) -> Option<(usize, Option<String>, Option<Vec<proc_macro2::TokenStream>>)> {
    for attr in attrs {
        if attr.path().is_ident("mock_data") {
            let result: Result<syn::punctuated::Punctuated<Meta, syn::Token![,]>, _> =
                attr.parse_args_with(syn::punctuated::Punctuated::parse_terminated);

            if let Ok(metas) = result {
                let mut n = 100; // default
                let mut overrides = None;

                for meta in metas {
                    match meta {
                        Meta::NameValue(nv) if nv.path.is_ident("n") => {
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Int(lit), ..
                            }) = &nv.value
                            {
                                n = lit.base10_parse().unwrap_or(100);
                            }
                        }
                        Meta::NameValue(nv) if nv.path.is_ident("overrides") => {
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Str(lit), ..
                            }) = &nv.value
                            {
                                overrides = Some(lit.value());
                            }
                        }
                        _ => {}
                    }
                }

                // Also parse coordinates
                let coordinates = coordinate_parsing::parse_coordinate_attribute(attrs);

                return Some((n, overrides, coordinates));
            }
        }
    }
    None
}

fn parse_table_validators(attrs: &[Attribute]) -> Vec<String> {
    let mut validators = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("validators") {
            let result: Result<syn::punctuated::Punctuated<Meta, syn::Token![,]>, _> =
                attr.parse_args_with(syn::punctuated::Punctuated::parse_terminated);

            if let Ok(metas) = result {
                for meta in metas {
                    match meta {
                        Meta::NameValue(nv) if nv.path.is_ident("custom") => {
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Str(lit), ..
                            }) = &nv.value
                            {
                                validators.push(lit.value());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    validators
}

fn parse_relation_attribute(
    attrs: &[Attribute],
) -> Option<helpers::evenframe::schemasync::EdgeConfig> {
    for attr in attrs {
        if attr.path().is_ident("relation") {
            let result: Result<syn::punctuated::Punctuated<Meta, syn::Token![,]>, _> =
                attr.parse_args_with(syn::punctuated::Punctuated::parse_terminated);

            if let Ok(metas) = result {
                let mut edge_name = None;
                let mut from = None;
                let mut to = None;
                let mut direction = None;

                for meta in metas {
                    match meta {
                        Meta::NameValue(nv) if nv.path.is_ident("edge_name") => {
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Str(lit), ..
                            }) = &nv.value
                            {
                                edge_name = Some(lit.value());
                            }
                        }
                        Meta::NameValue(nv) if nv.path.is_ident("from") => {
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Str(lit), ..
                            }) = &nv.value
                            {
                                from = Some(lit.value());
                            }
                        }
                        Meta::NameValue(nv) if nv.path.is_ident("to") => {
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Str(lit), ..
                            }) = &nv.value
                            {
                                to = Some(lit.value());
                            }
                        }
                        Meta::NameValue(nv) if nv.path.is_ident("direction") => {
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Str(lit), ..
                            }) = &nv.value
                            {
                                direction = match lit.value().as_str() {
                                    "from" => Some(helpers::evenframe::schemasync::Direction::From),
                                    "to" => Some(helpers::evenframe::schemasync::Direction::To),
                                    _ => None,
                                };
                            }
                        }
                        _ => {}
                    }
                }

                if let (Some(edge_name), Some(from), Some(to), Some(direction)) =
                    (edge_name, from, to, direction)
                {
                    return Some(helpers::evenframe::schemasync::EdgeConfig {
                        edge_name,
                        from,
                        to,
                        direction,
                    });
                }
            }
        }
    }
    None
}

fn parse_field_validators(attrs: &[Attribute]) -> Vec<proc_macro2::TokenStream> {
    for attr in attrs {
        if attr.path().is_ident("validators") {
            let result = attr.parse_args::<syn::ExprParen>();

            if let Ok(expr_paren) = result {
                // Parse the expression inside parentheses
                let mut validator_tokens = Vec::new();

                // Convert the expression to a string and parse validators
                let expr_str = quote! { #expr_paren }.to_string();

                // This is a simplified parser - in reality, we'd need more sophisticated parsing
                if expr_str.contains("string") {
                    // Parse string validators
                    if expr_str.contains("email") {
                        validator_tokens.push(quote! {
                            helpers::evenframe::validator::Validator::StringValidator(
                                helpers::evenframe::validator::StringValidator::Email
                            )
                        });
                    }
                    if expr_str.contains("date_iso") {
                        validator_tokens.push(quote! {
                            helpers::evenframe::validator::Validator::StringValidator(
                                helpers::evenframe::validator::StringValidator::DateIso
                            )
                        });
                    }
                    if let Some(regex_start) = expr_str.find("regex = \"") {
                        let regex_start = regex_start + 9;
                        if let Some(regex_end) = expr_str[regex_start..].find("\"") {
                            let regex = &expr_str[regex_start..regex_start + regex_end];
                            validator_tokens.push(quote! {
                                helpers::evenframe::validator::Validator::StringValidator(
                                    helpers::evenframe::validator::StringValidator::RegexLiteral(#regex.to_string())
                                )
                            });
                        }
                    }
                    if let Some(length_start) = expr_str.find("length = \"") {
                        let length_start = length_start + 10;
                        if let Some(length_end) = expr_str[length_start..].find("\"") {
                            let length = &expr_str[length_start..length_start + length_end];
                            validator_tokens.push(quote! {
                                helpers::evenframe::validator::Validator::StringValidator(
                                    helpers::evenframe::validator::StringValidator::Length(#length.to_string())
                                )
                            });
                        }
                    }
                }

                return validator_tokens;
            }
        }
    }
    vec![]
}

fn parse_format_attribute(attrs: &[Attribute]) -> Option<proc_macro2::TokenStream> {
    for attr in attrs {
        if attr.path().is_ident("format") {
            if let Ok(format_ident) = attr.parse_args::<syn::Ident>() {
                let _format_str = format_ident.to_string();
                return Some(quote! {
                    helpers::evenframe::schemasync::format::Format::#format_ident
                });
            }
        }
    }
    None
}

/// The merged procedural macro: `#[derive(Schemasync)]`
///
/// For structs it generates both:
/// - A `table_schema()` function returning a `helpers::TableSchema`, and
/// - CRUD async functions (`create`, `update`, `delete`, `read`, `fetch`)
///   that build JSON payloads and generate SQL query strings including:
///     - For fields with an `edge` attribute, subqueries in the SELECT clause.
///     - For fields with a `fetch` attribute, a FETCH clause listing the fetched field names.
///   If the struct does not contain an "id" field, the handler functions are omitted.
/// For enums it generates a `variants()` method returning a `TaggedUnion`.
#[proc_macro_derive(
    Schemasync,
    attributes(
        edge,
        fetch,
        define_field_statement,
        subquery,
        format,
        permissions,
        mock_data,
        validators,
        relation
    )
)]
pub fn schemasync_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;

    match input.data {
        Data::Struct(ref data_struct) => {
            // Ensure the struct has named fields.
            let fields_named = if let Fields::Named(ref fields_named) = data_struct.fields {
                fields_named
            } else {
                return syn::Error::new(
                    ident.span(),
                    "Schemasync only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            };

            // Parse struct-level attributes
            let permissions_config =
                match helpers::evenframe::schemasync::PermissionsConfig::parse(&input.attrs) {
                    Ok(config) => config,
                    Err(err) => return err.to_compile_error().into(),
                };

            // Parse mock_data attribute
            let mock_data_config = parse_mock_data_attribute(&input.attrs);

            // Parse table-level validators
            let table_validators = parse_table_validators(&input.attrs);

            // Parse relation attribute
            let relation_config = parse_relation_attribute(&input.attrs);

            // Check if an "id" field exists.
            let has_id = fields_named
                .named
                .iter()
                .any(|field| field.ident.as_ref().map(|id| id == "id").unwrap_or(false));

            // Single pass over all fields.
            let mut table_field_tokens = Vec::new();
            let mut json_assignments = Vec::new();
            let mut fetch_fields = Vec::new(); // For fields marked with #[fetch]
            let mut subqueries: Vec<String> = Vec::new();
            for field in fields_named.named.iter() {
                let field_ident = field.ident.as_ref().unwrap();
                let field_name = field_ident.to_string();
                let field_name_trim = field_name.trim_start_matches("r#");

                // Build the field type token.
                let ty = &field.ty;
                let field_type = parse_data_type(ty);

                // Parse any edge attribute.
                let edge_config = match helpers::evenframe::schemasync::EdgeConfig::parse(field) {
                    Ok(details) => details,
                    Err(err) => return err.to_compile_error().into(),
                };

                // Parse any define details.
                let define_config = match helpers::evenframe::schemasync::DefineConfig::parse(field)
                {
                    Ok(details) => details,
                    Err(err) => return err.to_compile_error().into(),
                };

                // Parse any format attribute.
                let format = parse_format_attribute(&field.attrs);

                // Parse field-level validators
                let field_validators = parse_field_validators(&field.attrs);

                // Parse any subquery attribute, overrides default edge subquery if found
                if let Some(subquery_value) = field
                    .attrs
                    .iter()
                    .find(|a| a.path().is_ident("subquery"))
                    .map(|attr| {
                        attr.parse_args::<LitStr>().map_err(|e| {
                            syn::Error::new(
                                attr.span(),
                                format!("Expected a string literal for subquery attribute: {e}"),
                            )
                        })
                    })
                    .transpose()
                    .expect("There was a problem parsing the subquery for the field {field_name}")
                {
                    subqueries.push(subquery_value.value());
                } else if let Some(ref details) = edge_config {
                    let subquery = if details.direction
                        == helpers::evenframe::schemasync::Direction::From
                    {
                        format!(
                            "(SELECT ->{}.* AS data FROM $parent.id FETCH data.out)[0].data as {}",
                            details.edge_name, field_name
                        )
                    } else if details.direction == helpers::evenframe::schemasync::Direction::To {
                        format!(
                            "(SELECT <-{}.* AS data FROM $parent.id FETCH data.in)[0].data as {}",
                            details.edge_name, field_name
                        )
                    } else {
                        "".to_string()
                    };

                    subqueries.push(subquery);
                }

                // Check for a fetch attribute.
                let has_fetch = field.attrs.iter().any(|a| a.path().is_ident("fetch"));

                // Build the schema token for this field.
                let edge_config_tokens = if let Some(ref details) = edge_config {
                    quote! {
                        Some(#details)
                    }
                } else {
                    quote! { None }
                };

                // Build the schema token for this field.
                let define_config_tokens = if let Some(ref define) = define_config {
                    quote! {
                        Some(#define)
                    }
                } else {
                    quote! { None }
                };

                // Build the schema token for this field.
                let format_tokens = if let Some(ref fmt) = format {
                    quote! { Some(#fmt) }
                } else {
                    quote! { None }
                };

                // Build validators token for this field
                let validators_tokens = if field_validators.is_empty() {
                    quote! { vec![] }
                } else {
                    quote! { vec![#(#field_validators),*] }
                };

                table_field_tokens.push(quote! {
                    helpers::evenframe::schemasync::TableField {
                        field_name: #field_name_trim.to_string(),
                        field_type: #field_type,
                        edge_config: #edge_config_tokens,
                        define_config: #define_config_tokens,
                        format: #format_tokens,
                        validators: #validators_tokens,
                        always_regenerate: false
                    }
                });

                // For the JSON payload, skip the "id" field and any field with an edge attribute.
                if field_name != "id" && edge_config.is_none() {
                    json_assignments.push(quote! {
                        #field_name: payload.#field_ident,
                    });
                }

                // If the field has a fetch attribute, add its name for the FETCH clause.
                if has_fetch {
                    fetch_fields.push(field_name);
                }
            }

            // Build the JSON payload block.
            // let json_payload = quote! { { #(#json_assignments)* } };

            // Create the edge subquery string fragments.
            let edge_query_part_read = if !subqueries.is_empty() {
                format!(", {}", subqueries.join(", "))
            } else {
                "".to_string()
            };
            let edge_query_part_fetch = if !subqueries.is_empty() {
                format!(", {}", subqueries.join(", "))
            } else {
                "".to_string()
            };

            // Create a FETCH clause for any fetch fields.
            let fetch_clause = if !fetch_fields.is_empty() {
                format!(" FETCH {}", fetch_fields.join(", "))
            } else {
                "".to_string()
            };

            // Generate tokens for parsed attributes (shared between implementations)
            let struct_name = ident.to_string();

            let permissions_config_tokens = if let Some(ref config) = permissions_config {
                quote! { Some(#config) }
            } else {
                quote! { None }
            };

            let table_validators_tokens = if !table_validators.is_empty() {
                let validator_strings = table_validators.iter().map(|v| quote! { #v.to_string() });
                quote! {
                    vec![
                        #(helpers::evenframe::validator::Validator::StringValidator(
                            helpers::evenframe::validator::StringValidator::StringEmbedded(#validator_strings)
                        )),*
                    ]
                }
            } else {
                quote! { vec![] }
            };

            let mock_data_tokens = if let Some((n, _overrides, coordinates)) = mock_data_config {
                let coord_rules = if let Some(coords) = coordinates {
                    quote! { vec![#(#coords),*] }
                } else {
                    quote! { vec![] }
                };

                quote! {
                    Some(helpers::evenframe::schemasync::mock::MockGenerationConfig {
                        n: #n,
                        table_level_override: None, // TODO: parse overrides
                        coordination_rules: #coord_rules,
                        preserve_unchanged: false,
                        preserve_modified: false,
                        batch_size: 1000,
                        regenerate_fields: vec!["updated_at".to_string(), "created_at".to_string()],
                        preservation_mode: helpers::evenframe::schemasync::merge::PreservationMode::None,
                    })
                }
            } else {
                quote! { None }
            };

            let relation_tokens = if let Some(ref rel) = relation_config {
                quote! { Some(#rel) }
            } else {
                quote! { None }
            };

            let evenframe_persistable_struct_impl = {
                quote! {
                    impl helpers::evenframe::traits::EvenframePersistableStruct for #ident {
                        fn name() -> String {
                            #struct_name.to_string()
                        }

                        fn validators() -> Vec<helpers::evenframe::validator::Validator> {
                            #table_validators_tokens
                        }

                        fn permissions_config() -> Option<helpers::evenframe::schemasync::PermissionsConfig> {
                            #permissions_config_tokens
                        }

                        fn struct_config() -> helpers::evenframe::schemasync::StructConfig {
                            helpers::evenframe::schemasync::StructConfig {
                                name: helpers::case::to_snake_case(#struct_name),
                                fields: vec![ #(#table_field_tokens),* ],
                                validators: vec![],
                            }
                        }

                        fn table_fields() -> Vec<helpers::evenframe::schemasync::TableField> {
                            vec![ #(#table_field_tokens),* ]
                        }

                        fn table_config() -> Option<helpers::evenframe::schemasync::TableConfig> {
                            Some(helpers::evenframe::schemasync::TableConfig {
                                struct_config: helpers::evenframe::schemasync::StructConfig {
                                    name: helpers::case::to_snake_case(#struct_name),
                                    fields: vec![ #(#table_field_tokens),* ],
                                    validators: vec![],
                                },
                                relation: #relation_tokens,
                                permissions: #permissions_config_tokens,
                                mock_generation_config: Self::mock_generation_config(),
                            })
                        }

                        fn get_table_config(&self) -> Option<helpers::evenframe::schemasync::TableConfig> {
                            Self::table_config()
                        }


                        fn mock_generation_config() -> Option<helpers::evenframe::schemasync::mock::MockGenerationConfig> {
                            #mock_data_tokens
                        }

                        fn router() -> axum::Router<helpers::app_state::AppState> {
                            let collection_name = helpers::case::to_snake_case(&#struct_name);
                            let collection_name_plural = format!("{}s", collection_name);

                            axum::Router::new()
                                .route(&format!("/{}", collection_name), axum::routing::post(helpers::create!(#ident)))
                                .route(&format!("/{}", collection_name), axum::routing::put(helpers::update!(#ident)))
                                .route(&format!("/{}", collection_name), axum::routing::delete(helpers::delete!(#ident)))
                                .route(&format!("/{}", collection_name), axum::routing::get(helpers::read!(#ident)))
                                .route(&format!("/{}", collection_name_plural), axum::routing::get(helpers::fetch!(#ident)))
                        }
                    }
                }
            };

            // If the struct has an "id" field, generate the handler closures.
            let handlers_impl = if has_id {
                let collection_name = helpers::case::to_snake_case(&ident.to_string());
                let collection_name_lit = LitStr::new(&collection_name, ident.span());
                let query_read1 = format!("SELECT *{} from ", edge_query_part_read);
                let query_read2 = format!("{};", fetch_clause);
                let query_fetch = format!(
                    "SELECT *{} from {}{};",
                    edge_query_part_fetch, collection_name, fetch_clause
                );
                let query_read1_lit = LitStr::new(&query_read1, ident.span());
                let query_read2_lit = LitStr::new(&query_read2, ident.span());
                let query_fetch_lit = LitStr::new(&query_fetch, ident.span());
                let create_fn = syn::Ident::new("create", ident.span());
                let update_fn = syn::Ident::new("update", ident.span());
                let delete_fn = syn::Ident::new("delete", ident.span());
                let read_fn = syn::Ident::new("read", ident.span());
                let fetch_fn = syn::Ident::new("fetch", ident.span());

                quote! {

                    pub async fn #create_fn(
                        State(state): State<helpers::app_state::AppState>,
                        jar: axum_extra::extract::PrivateCookieJar,
                        axum_extra::TypedHeader(host): axum_extra::TypedHeader<headers::Host>,
                        Json(payload): Json<#ident>,
                    ) -> Result<(StatusCode, Json<#ident>), Error> {
                        dotenv::dotenv().ok(); // Load .env file
                        let query = helpers::evenframe::schemasync::generate_query(helpers::evenframe::schemasync::QueryType::Create, &payload.get_table_config().unwrap(), &payload, None);
                        let _ = ureq::post("http://localhost:8000/sql")
                        .header("Authorization", &format!("Bearer {}", &jar.get("auth_token").unwrap().to_string()[11..]))
                            .header("Accept", "application/json")
                            .header(
                                "Surreal-NS",
                                std::env::var("SURREAL_NAMESPACE").expect("Surreal namespace not set"),
                            )
                            .header(
                                "Surreal-DB",
                                std::env::var("SURREAL_DATABASE").expect("Surreal database not set"),
                            )
                        .send(query)
                        .unwrap()
                        .body_mut()
                        .read_json::<Vec<helpers::database::Response<serde_json::Value>>>().unwrap();

                        let item = #ident::#read_fn(State(state), jar, axum_extra::TypedHeader(host), Json(payload.id))
                            .await?
                            .1;
                        Ok((StatusCode::OK, item))
                    }


                    pub async fn #update_fn(
                        State(state): State<helpers::app_state::AppState>,
                        jar: axum_extra::extract::PrivateCookieJar,
                        axum_extra::TypedHeader(host): axum_extra::TypedHeader<headers::Host>,
                        Json(payload): Json<#ident>,
                    ) -> Result<(StatusCode, Json<#ident>), Error> {
                        dotenv::dotenv().ok(); // Load .env file
                        let query = helpers::evenframe::schemasync::generate_query(helpers::evenframe::schemasync::QueryType::Update, &payload.get_table_config().unwrap(), &payload, None);
                        let subdomain = &helpers::subdomain::Subdomain::get_subdomain(&host);
                        let db_name = &state.clients.get(subdomain).unwrap().0;

                        let _ = ureq::post("http://localhost:8000/sql")
                        .header("Authorization", &format!("Bearer {}", &jar.get("auth_token").unwrap().to_string()[11..]))
                            .header("Accept", "application/json")
                            .header(
                                "Surreal-NS",
                                std::env::var("SURREAL_NAMESPACE").expect("Surreal namespace not set"),
                            )
                            .header(
                                "Surreal-DB",
                                db_name,
                            )
                        .send(query)
                        .unwrap()
                        .body_mut()
                        .read_json::<Vec<helpers::database::Response<serde_json::Value>>>().unwrap();

                        let item = #ident::#read_fn(State(state), jar, axum_extra::TypedHeader(host), Json(payload.id))
                            .await?
                            .1;

                        Ok((StatusCode::OK, item))
                    }


                    pub async fn #delete_fn(
                        State(state): State<helpers::app_state::AppState>,
                        axum_extra::TypedHeader(host): axum_extra::TypedHeader<headers::Host>,
                        Json(payload): Json<surrealdb::RecordId>,
                    ) -> Result<(StatusCode, Json<#ident>), Error> {
                        let subdomain = &helpers::subdomain::Subdomain::get_subdomain(&host);
                        let db = &state.clients.get(subdomain).unwrap().1;


                        let item: #ident = db
                            .delete((#collection_name_lit, payload.to_string()))
                            .await?
                            .ok_or(Error::Db)?;
                        Ok((StatusCode::OK, Json(item)))
                    }


                    pub async fn #read_fn(
                        State(state): State<helpers::app_state::AppState>,
                        jar: axum_extra::extract::PrivateCookieJar,
                        axum_extra::TypedHeader(host): axum_extra::TypedHeader<headers::Host>,
                        Json(payload): Json<helpers::evenframe::wrappers::EvenframeRecordId>,
                    ) -> Result<(StatusCode, Json<#ident>), Error> {
                        dotenv::dotenv().ok(); // Load .env file
                        let query = format!("{}{}{}", #query_read1_lit, payload.to_string().replace("⟩", "").replace("⟨", ""), #query_read2_lit);
                        let subdomain = &helpers::subdomain::Subdomain::get_subdomain(&host);
                        let db = &state.clients.get(subdomain).unwrap().1;
                        let db_name = &state.clients.get(subdomain).unwrap().0;



                        db.set("payload", payload.to_string().replace("⟩", "").replace("⟨", "")).await?;
                        let response = ureq::post("http://localhost:8000/sql")
                        .header("Authorization", &format!("Bearer {}", &jar.get("auth_token").unwrap().to_string()[11..]))
                            .header("Accept", "application/json")
                            .header(
                                "Surreal-NS",
                                std::env::var("SURREAL_NAMESPACE").expect("Surreal namespace not set"),
                            )
                            .header(
                                "Surreal-DB",
                                db_name,
                            )
                        .send(query)
                        .unwrap()
                        .body_mut()
                        .read_json::<Vec<helpers::database::Response<#ident>>>().unwrap();
                        db.unset("payload").await?;


                        helpers::utils::log(&format!("{:?}", response[0].result.clone()), "logs/debug.log", true);

                        Ok((StatusCode::OK, Json(response[0].result[0].clone())))
                    }


                    pub async fn #fetch_fn(
                        State(state): State<helpers::app_state::AppState>,
                        jar: axum_extra::extract::PrivateCookieJar,
                        axum_extra::TypedHeader(host): axum_extra::TypedHeader<headers::Host>,
                    ) -> Result<(StatusCode, Json<Vec<#ident>>), Error> {
                        dotenv::dotenv().ok(); // Load .env file
                        let query = #query_fetch_lit;
                        let subdomain = &helpers::subdomain::Subdomain::get_subdomain(&host);
                        let db_name = &state.clients.get(subdomain).unwrap().0;

                        let response = ureq::post("http://localhost:8000/sql")
                                .header("Authorization", &format!("Bearer {}", &jar.get("auth_token").unwrap().to_string()[11..]))
                                .header("Accept", "application/json")
                                .header(
                                    "Surreal-NS",
                                    std::env::var("SURREAL_NAMESPACE").expect("Surreal namespace not set"),
                                )
                                .header(
                                    "Surreal-DB",
                                    db_name,
                                )
                                .send(format!("{}", query))
                                .unwrap()
                                .body_mut()
                                .read_json::<Vec<helpers::database::Response<#ident>>>().unwrap();


                        helpers::utils::log(&format!("{:?}", response[0].result.clone()), "logs/debug.log", true);

                        Ok((StatusCode::OK, Json(response[0].result.clone())))
                    }
                }
            } else {
                // No "id" field: only generate the schema.
                quote! {}
            };

            // Generate EvenframeAppStruct trait implementation
            let evenframe_app_struct_impl = {
                quote! {
                    impl helpers::evenframe::traits::EvenframeAppStruct for #ident {
                        fn name() -> String {
                            #struct_name.to_string()
                        }

                        fn struct_config() -> helpers::evenframe::schemasync::StructConfig {
                            helpers::evenframe::schemasync::StructConfig {
                                name: helpers::case::to_snake_case(#struct_name),
                                fields: vec![ #(#table_field_tokens),* ],
                                validators: vec![],
                            }
                        }

                        fn table_fields() -> Vec<helpers::evenframe::schemasync::TableField> {
                            vec![ #(#table_field_tokens),* ]
                        }
                    }
                }
            };

            let output = if has_id {
                quote! {
                    // Import the trait so it's available for method calls
                    use helpers::evenframe::traits::EvenframePersistableStruct as _;

                    #evenframe_persistable_struct_impl
                    impl #ident {
                        #handlers_impl
                    }
                }
            } else {
                quote! {
                    // Import the trait so it's available for method calls
                    use helpers::evenframe::traits::EvenframeAppStruct as _;

                    #evenframe_app_struct_impl
                }
            };

            output.into()
        }
        Data::Enum(ref data_enum) => {
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
                                quote! { Some(#field_type) }
                            } else {
                                let field_types =
                                    fields.unnamed.iter().map(|f| parse_data_type(&f.ty));
                                quote! { Some(helpers::evenframe::schemasync::FieldType::Tuple(vec![ #(#field_types),* ])) }
                            }
                        }
                        syn::Fields::Named(fields) => {
                            let field_types = fields.named.iter().map(|f| {
                                let fname = f.ident.as_ref().unwrap().to_string();
                                let ftype = parse_data_type(&f.ty);
                                quote! { (#fname.to_string(), #ftype) }
                            });
                            quote! { Some(helpers::evenframe::schemasync::FieldType::Struct(vec![ #(#field_types),* ])) }
                        }
                    };

                    quote! {
                        helpers::evenframe::schemasync::Variant {
                            name: #variant_name.to_string(),
                            data: #data_tokens,
                        }
                    }
                })
                .collect();

            let enum_impl = quote! {
                impl #ident {
                    pub fn variants() -> helpers::evenframe::schemasync::TaggedUnion {
                        helpers::evenframe::schemasync::TaggedUnion {
                            enum_name: #enum_name_lit.to_string(),
                            variants: vec![ #(#variant_tokens),* ],
                        }
                    }
                }
            };

            // Generate EvenframeEnum trait implementation
            let evenframe_enum_impl = quote! {
                impl helpers::evenframe::traits::EvenframeEnum for #ident {
                    fn name() -> String {
                        #enum_name_lit.to_string()
                    }

                    fn variants() -> Vec<helpers::evenframe::schemasync::Variant> {
                        vec![ #(#variant_tokens),* ]
                    }

                    fn tagged_union() -> helpers::evenframe::schemasync::TaggedUnion {
                        #ident::variants()
                    }
                }
            };

            let output = quote! {
                // Import the trait so it's available for method calls
                use helpers::evenframe::traits::EvenframeEnum as _;

                #enum_impl
                #evenframe_enum_impl
            };

            output.into()
        }

        _ => syn::Error::new(
            ident.span(),
            "Schemasync can only be used on structs and enums",
        )
        .to_compile_error()
        .into(),
    }
}
