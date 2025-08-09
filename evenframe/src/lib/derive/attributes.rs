use quote::quote;
use syn::{spanned::Spanned, Attribute, Expr, ExprLit, Lit, Meta};

use crate::{
    derive::coordinate_parser::parse_coordinate_attribute,
    format::Format,
    schemasync::{Direction, EdgeConfig},
};

// Remove unused imports - these are only used in the macro implementation, not generated code

pub fn parse_mock_data_attribute(
    attrs: &[Attribute],
) -> Result<Option<(usize, Option<String>, Option<Vec<proc_macro2::TokenStream>>)>, syn::Error> {
    for attr in attrs {
        if attr.path().is_ident("mock_data") {
            let result: Result<syn::punctuated::Punctuated<Meta, syn::Token![,]>, _> =
                attr.parse_args_with(syn::punctuated::Punctuated::parse_terminated);

            match result {
                Ok(metas) => {
                    let mut n = 100; // default
                    let mut overrides = None;

                    for meta in metas {
                        match meta {
                            Meta::NameValue(nv) if nv.path.is_ident("n") => {
                                if let Expr::Lit(ExprLit {
                                    lit: Lit::Int(lit), ..
                                }) = &nv.value
                                {
                                    match lit.base10_parse::<usize>() {
                                        Ok(value) => n = value,
                                        Err(_) => {
                                            return Err(syn::Error::new(
                                                lit.span(),
                                                format!("Invalid value for 'n': '{}'. Expected a positive integer.\n\nExample: #[mock_data(n = 1000)]", lit.base10_digits())
                                            ));
                                        }
                                    }
                                } else {
                                    return Err(syn::Error::new(
                                        nv.value.span(),
                                        "The 'n' parameter must be an integer literal.\n\nExample: #[mock_data(n = 1000)]"
                                    ));
                                }
                            }
                            Meta::NameValue(nv) if nv.path.is_ident("overrides") => {
                                if let Expr::Lit(ExprLit {
                                    lit: Lit::Str(lit), ..
                                }) = &nv.value
                                {
                                    overrides = Some(lit.value());
                                } else {
                                    return Err(syn::Error::new(
                                        nv.value.span(),
                                        "The 'overrides' parameter must be a string literal.\n\nExample: #[mock_data(overrides = \"custom_config\")]"
                                    ));
                                }
                            }
                            Meta::NameValue(nv) if nv.path.is_ident("coordinate") => {
                                // Skip here - coordinate is parsed separately by coordinate_parser
                            }
                            Meta::NameValue(nv) => {
                                let param_name = nv
                                    .path
                                    .get_ident()
                                    .map(|i| i.to_string())
                                    .unwrap_or_else(|| "unknown".to_string());
                                return Err(syn::Error::new(
                                    nv.path.span(),
                                    format!("Unknown parameter '{}' in mock_data attribute.\n\nValid parameters are: n, overrides, coordinate\n\nExample: #[mock_data(n = 1000, overrides = \"config\", coordinate = [InitializeEqual([\"field1\", \"field2\"])])]", param_name)
                                ));
                            }
                            _ => {
                                return Err(syn::Error::new(
                                    meta.span(),
                                    "Invalid syntax in mock_data attribute.\n\nExpected format: #[mock_data(n = 1000, overrides = \"config\")]"
                                ));
                            }
                        }
                    }

                    // Also parse coordinates
                    let coordinates = match parse_coordinate_attribute(attrs) {
                        Ok(coords) => coords,
                        Err(err) => return Err(err),
                    };

                    return Ok(Some((n, overrides, coordinates)));
                }
                Err(err) => {
                    return Err(syn::Error::new(
                        attr.span(),
                        format!("Failed to parse mock_data attribute: {}\n\nExample usage:\n#[mock_data(n = 1000)]\n#[mock_data(n = 500, overrides = \"custom_config\")]", err)
                    ));
                }
            }
        }
    }
    Ok(None)
}

pub fn parse_table_validators(attrs: &[Attribute]) -> Result<Vec<String>, syn::Error> {
    let mut validators = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("validators") {
            let result: Result<syn::punctuated::Punctuated<Meta, syn::Token![,]>, _> =
                attr.parse_args_with(syn::punctuated::Punctuated::parse_terminated);

            match result {
                Ok(metas) => {
                    for meta in metas {
                        match meta {
                            Meta::NameValue(nv) if nv.path.is_ident("custom") => {
                                if let Expr::Lit(ExprLit {
                                    lit: Lit::Str(lit), ..
                                }) = &nv.value
                                {
                                    validators.push(lit.value());
                                } else {
                                    return Err(syn::Error::new(
                                        nv.value.span(),
                                        "The 'custom' parameter must be a string literal containing a validation expression.\n\nExample: #[validators(custom = \"$value > 0 AND $value < 100\")]"
                                    ));
                                }
                            }
                            Meta::NameValue(nv) => {
                                let param_name = nv
                                    .path
                                    .get_ident()
                                    .map(|i| i.to_string())
                                    .unwrap_or_else(|| "unknown".to_string());
                                return Err(syn::Error::new(
                                    nv.path.span(),
                                    format!("Unknown parameter '{}' in validators attribute.\n\nValid parameter is: custom\n\nExample: #[validators(custom = \"$value > 0\")]", param_name)
                                ));
                            }
                            _ => {
                                return Err(syn::Error::new(
                                    meta.span(),
                                    "Invalid syntax in validators attribute.\n\nExpected format: #[validators(custom = \"validation_expression\")]"
                                ));
                            }
                        }
                    }
                }
                Err(err) => {
                    return Err(syn::Error::new(
                        attr.span(),
                        format!("Failed to parse validators attribute: {}\n\nExample usage:\n#[validators(custom = \"$value > 0\")]\n#[validators(custom = \"string::len($value) > 5\")]", err)
                    ));
                }
            }
        }
    }

    Ok(validators)
}

pub fn parse_relation_attribute(attrs: &[Attribute]) -> Result<Option<EdgeConfig>, syn::Error> {
    for attr in attrs {
        if attr.path().is_ident("relation") {
            let result: Result<syn::punctuated::Punctuated<Meta, syn::Token![,]>, _> =
                attr.parse_args_with(syn::punctuated::Punctuated::parse_terminated);

            match result {
                Ok(metas) => {
                    let mut edge_name = None;
                    let mut from = None;
                    let mut to = None;
                    let mut direction: Option<Direction> = None;

                    for meta in metas {
                        match meta {
                            Meta::NameValue(nv) if nv.path.is_ident("edge_name") => {
                                if let Expr::Lit(ExprLit {
                                    lit: Lit::Str(lit), ..
                                }) = &nv.value
                                {
                                    edge_name = Some(lit.value());
                                } else {
                                    return Err(syn::Error::new(
                                        nv.value.span(),
                                        "The 'edge_name' parameter must be a string literal.\n\nExample: edge_name = \"has_user\""
                                    ));
                                }
                            }
                            Meta::NameValue(nv) if nv.path.is_ident("from") => {
                                if let Expr::Lit(ExprLit {
                                    lit: Lit::Str(lit), ..
                                }) = &nv.value
                                {
                                    from = Some(lit.value());
                                } else {
                                    return Err(syn::Error::new(
                                        nv.value.span(),
                                        "The 'from' parameter must be a string literal.\n\nExample: from = \"Order\""
                                    ));
                                }
                            }
                            Meta::NameValue(nv) if nv.path.is_ident("to") => {
                                if let Expr::Lit(ExprLit {
                                    lit: Lit::Str(lit), ..
                                }) = &nv.value
                                {
                                    to = Some(lit.value());
                                } else {
                                    return Err(syn::Error::new(
                                        nv.value.span(),
                                        "The 'to' parameter must be a string literal.\n\nExample: to = \"User\""
                                    ));
                                }
                            }
                            Meta::NameValue(nv) if nv.path.is_ident("direction") => {
                                if let Expr::Lit(ExprLit {
                                    lit: Lit::Str(lit), ..
                                }) = &nv.value
                                {
                                    direction = match lit.value().as_str() {
                                        "from" => Some(Direction::From),
                                        "to" => Some(Direction::To),
                                        other => {
                                            return Err(syn::Error::new(
                                                lit.span(),
                                                format!("Invalid direction '{}'. Valid values are: \"from\", \"to\"\n\nExample: direction = \"from\"", other)
                                            ));
                                        }
                                    };
                                } else {
                                    return Err(syn::Error::new(
                                        nv.value.span(),
                                        "The 'direction' parameter must be a string literal with value \"from\" or \"to\".\n\nExample: direction = \"from\""
                                    ));
                                }
                            }
                            Meta::NameValue(nv) => {
                                let param_name = nv
                                    .path
                                    .get_ident()
                                    .map(|i| i.to_string())
                                    .unwrap_or_else(|| "unknown".to_string());
                                return Err(syn::Error::new(
                                    nv.path.span(),
                                    format!("Unknown parameter '{}' in relation attribute.\n\nValid parameters are: edge_name, from, to, direction\n\nExample: #[relation(edge_name = \"has_user\", from = \"Order\", to = \"User\", direction = \"from\")]", param_name)
                                ));
                            }
                            _ => {
                                return Err(syn::Error::new(
                                    meta.span(),
                                    "Invalid syntax in relation attribute.\n\nExpected format: #[relation(edge_name = \"...\", from = \"...\", to = \"...\", direction = \"...\")]"
                                ));
                            }
                        }
                    }

                    match (&edge_name, &from, &to, &direction) {
                        (Some(edge_name), Some(from), Some(to), Some(direction)) => {
                            return Ok(Some(EdgeConfig {
                                edge_name: edge_name.to_owned(),
                                from: from.to_owned(),
                                to: to.to_owned(),
                                direction: direction.to_owned(),
                            }));
                        }
                        _ => {
                            let missing = vec![
                                edge_name.is_none().then(|| "edge_name"),
                                from.is_none().then(|| "from"),
                                to.is_none().then(|| "to"),
                                direction.is_none().then(|| "direction"),
                            ]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>()
                            .join(", ");

                            return Err(syn::Error::new(
                                attr.span(),
                                format!("Missing required parameters in relation attribute: {}\n\nAll parameters are required:\n#[relation(\n    edge_name = \"has_user\",\n    from = \"Order\",\n    to = \"User\",\n    direction = \"from\"\n)]", missing)
                            ));
                        }
                    }
                }
                Err(err) => {
                    return Err(syn::Error::new(
                        attr.span(),
                        format!("Failed to parse relation attribute: {}\n\nExample usage:\n#[relation(\n    edge_name = \"has_user\",\n    from = \"Order\",\n    to = \"User\",\n    direction = \"from\"\n)]", err)
                    ));
                }
            }
        }
    }
    Ok(None)
}

pub fn parse_format_attribute(
    attrs: &[Attribute],
) -> Result<Option<proc_macro2::TokenStream>, syn::Error> {
    use syn::{Expr, ExprCall, ExprPath, Path, PathSegment};

    for attr in attrs {
        if attr.path().is_ident("format") {
            // Parse the attribute content as an expression
            let expr: syn::Expr = attr.parse_args()
                .map_err(|e| syn::Error::new(
                    attr.span(),
                    format!("Failed to parse format attribute: {}\n\nExamples:\n#[format(DateTime)]\n#[format(Url(\"example.com\"))]", e)
                ))?;

            // Transform the expression to add Format:: prefix if needed
            let format_expr = match &expr {
                // If it's just an identifier like DateTime, convert to Format::DateTime
                Expr::Path(path_expr) if path_expr.path.segments.len() == 1 => {
                    let variant = &path_expr.path.segments[0];
                    let mut segments = syn::punctuated::Punctuated::new();
                    segments.push(PathSegment::from(syn::Ident::new("Format", variant.span())));
                    segments.push(variant.clone());
                    Expr::Path(ExprPath {
                        attrs: vec![],
                        qself: None,
                        path: Path {
                            leading_colon: None,
                            segments,
                        },
                    })
                }
                // If it's a call like Url("domain"), convert to Format::Url("domain")
                Expr::Call(call_expr) => {
                    if let Expr::Path(path_expr) = &*call_expr.func {
                        if path_expr.path.segments.len() == 1 {
                            let variant = &path_expr.path.segments[0];
                            let mut segments = syn::punctuated::Punctuated::new();
                            segments
                                .push(PathSegment::from(syn::Ident::new("Format", variant.span())));
                            segments.push(variant.clone());
                            Expr::Call(ExprCall {
                                attrs: call_expr.attrs.clone(),
                                func: Box::new(Expr::Path(ExprPath {
                                    attrs: vec![],
                                    qself: None,
                                    path: Path {
                                        leading_colon: None,
                                        segments,
                                    },
                                })),
                                paren_token: call_expr.paren_token,
                                args: call_expr.args.clone(),
                            })
                        } else {
                            expr.clone()
                        }
                    } else {
                        expr.clone()
                    }
                }
                // Otherwise keep as is
                _ => expr.clone(),
            };

            // Use the TryFrom implementation to parse the Format
            match Format::try_from(&format_expr) {
                Ok(format) => {
                    // Since Format implements ToTokens, we can just quote it directly
                    return Ok(Some(quote! { #format }));
                }
                Err(e) => {
                    return Err(syn::Error::new(
                        expr.span(),
                        format!("{}\n\nValid formats:\n- Simple: DateTime, Date, Time, Currency, Percentage, Phone, Email, FirstName, LastName, CompanyName, PhoneNumber, ColorHex, JwtToken, Oklch, PostalCode\n- With parameter: Url(\"domain.com\")", e)
                    ));
                }
            }
        }
    }
    Ok(None)
}
