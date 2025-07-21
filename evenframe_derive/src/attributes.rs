use quote::quote;
use syn::{Attribute, Expr, ExprLit, Lit, Meta};

pub fn parse_mock_data_attribute(
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
                let coordinates = crate::coordinate_parsing::parse_coordinate_attribute(attrs);

                return Some((n, overrides, coordinates));
            }
        }
    }
    None
}

pub fn parse_table_validators(attrs: &[Attribute]) -> Vec<String> {
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

pub fn parse_relation_attribute(
    attrs: &[Attribute],
) -> Option<::helpers::evenframe::schemasync::EdgeConfig> {
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
                                    "from" => Some(::helpers::evenframe::schemasync::Direction::From),
                                    "to" => Some(::helpers::evenframe::schemasync::Direction::To),
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
                    return Some(::helpers::evenframe::schemasync::EdgeConfig {
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

pub fn parse_format_attribute(attrs: &[Attribute]) -> Option<proc_macro2::TokenStream> {
    for attr in attrs {
        if attr.path().is_ident("format") {
            if let Ok(format_ident) = attr.parse_args::<syn::Ident>() {
                let _format_str = format_ident.to_string();
                return Some(quote! {
                    ::helpers::evenframe::schemasync::format::Format::#format_ident
                });
            }
        }
    }
    None
}