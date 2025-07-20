use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Expr, ExprArray, ExprLit, Lit, Meta};

pub fn parse_coordinate_attribute(attrs: &[Attribute]) -> Option<Vec<TokenStream>> {
    for attr in attrs {
        if attr.path().is_ident("mock_data") {
            let result: Result<syn::punctuated::Punctuated<Meta, syn::Token![,]>, _> =
                attr.parse_args_with(syn::punctuated::Punctuated::parse_terminated);

            if let Ok(metas) = result {
                for meta in metas {
                    match meta {
                        Meta::NameValue(nv) if nv.path.is_ident("coordinate") => {
                            // coordinate = [...]
                            if let Expr::Array(ExprArray { elems, .. }) = &nv.value {
                                let mut coordinates = Vec::new();

                                for elem in elems {
                                    if let Some(coord_tokens) = parse_coordinate_expr(elem) {
                                        coordinates.push(coord_tokens);
                                    }
                                }

                                return Some(coordinates);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    None
}

fn parse_coordinate_expr(expr: &Expr) -> Option<TokenStream> {
    match expr {
        Expr::Call(call) => {
            let func_name = if let Expr::Path(path) = &*call.func {
                path.path.segments.last()?.ident.to_string()
            } else {
                return None;
            };

            match func_name.as_str() {
                "InitializeEqual" => {
                    // InitializeEqual(["field1", "field2"])
                    if let Some(Expr::Array(arr)) = call.args.first() {
                        let fields = parse_string_array(arr)?;
                        return Some(quote! {
                            helpers::schemasync::coordinate::Coordination::InitializeEqual(vec![#(#fields.to_string()),*])
                        });
                    }
                }
                "InitializeSequential" => {
                    // InitializeSequential { fields: [...], increment: Days(7) }
                    if let Some(Expr::Struct(s)) = call.args.first() {
                        let fields = extract_field_value(&s.fields, "fields")
                            .and_then(|e| {
                                if let Expr::Array(arr) = e {
                                    Some(arr)
                                } else {
                                    None
                                }
                            })
                            .and_then(parse_string_array)?;

                        let increment = extract_field_value(&s.fields, "increment")
                            .and_then(parse_increment)?;

                        return Some(quote! {
                            helpers::schemasync::coordinate::Coordination::InitializeSequential {
                                fields: vec![#(#fields.to_string()),*],
                                increment: #increment,
                            }
                        });
                    }
                }
                "InitializeSum" => {
                    // InitializeSum { fields: [...], total: 100.0 }
                    if let Some(Expr::Struct(s)) = call.args.first() {
                        let fields = extract_field_value(&s.fields, "fields")
                            .and_then(|e| {
                                if let Expr::Array(arr) = e {
                                    Some(arr)
                                } else {
                                    None
                                }
                            })
                            .and_then(parse_string_array)?;

                        let total = extract_field_value(&s.fields, "total").and_then(|e| {
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Float(f), ..
                            }) = e
                            {
                                Some(f.base10_parse::<f64>().ok()?)
                            } else {
                                None
                            }
                        })?;

                        return Some(quote! {
                            helpers::schemasync::coordinate::Coordination::InitializeSum {
                                fields: vec![#(#fields.to_string()),*],
                                total: #total,
                            }
                        });
                    }
                }
                "InitializeCoherent" => {
                    // InitializeCoherent { fields: [...], dataset: Address }
                    if let Some(Expr::Struct(s)) = call.args.first() {
                        let fields = extract_field_value(&s.fields, "fields")
                            .and_then(|e| {
                                if let Expr::Array(arr) = e {
                                    Some(arr)
                                } else {
                                    None
                                }
                            })
                            .and_then(parse_string_array)?;

                        let dataset = extract_field_value(&s.fields, "dataset")
                            .and_then(parse_coherent_dataset)?;

                        return Some(quote! {
                            helpers::schemasync::coordinate::Coordination::InitializeCoherent {
                                fields: vec![#(#fields.to_string()),*],
                                dataset: #dataset,
                            }
                        });
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    None
}

fn parse_string_array(arr: &ExprArray) -> Option<Vec<String>> {
    let mut strings = Vec::new();
    for elem in &arr.elems {
        if let Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) = elem
        {
            strings.push(s.value());
        } else {
            return None;
        }
    }
    Some(strings)
}

fn extract_field_value<'a>(
    fields: &'a syn::punctuated::Punctuated<syn::FieldValue, syn::Token![,]>,
    name: &str,
) -> Option<&'a Expr> {
    for field in fields {
        if let syn::Member::Named(ident) = &field.member {
            if ident == name {
                return Some(&field.expr);
            }
        }
    }
    None
}

fn parse_increment(expr: &Expr) -> Option<TokenStream> {
    if let Expr::Call(call) = expr {
        let func_name = if let Expr::Path(path) = &*call.func {
            path.path.segments.last()?.ident.to_string()
        } else {
            return None;
        };

        if let Some(Expr::Lit(ExprLit {
            lit: Lit::Int(n), ..
        })) = call.args.first()
        {
            let value: i32 = n.base10_parse().ok()?;

            match func_name.as_str() {
                "Days" => {
                    return Some(
                        quote! { helpers::schemasync::coordinate::CoordinateIncrement::Days(#value) },
                    )
                }
                "Hours" => {
                    return Some(
                        quote! { helpers::schemasync::coordinate::CoordinateIncrement::Hours(#value) },
                    )
                }
                "Minutes" => {
                    return Some(
                        quote! { helpers::schemasync::coordinate::CoordinateIncrement::Minutes(#value) },
                    )
                }
                _ => {}
            }
        }

        if let Some(Expr::Lit(ExprLit {
            lit: Lit::Float(f), ..
        })) = call.args.first()
        {
            let value: f64 = f.base10_parse().ok()?;
            if func_name == "Numeric" {
                return Some(
                    quote! { helpers::schemasync::coordinate::CoordinateIncrement::Numeric(#value) },
                );
            }
        }
    }
    None
}

fn parse_coherent_dataset(expr: &Expr) -> Option<TokenStream> {
    if let Expr::Path(path) = expr {
        let dataset_name = path.path.segments.last()?.ident.to_string();

        match dataset_name.as_str() {
            "Address" => {
                return Some(quote! { helpers::schemasync::coordinate::CoherentDataset::Address })
            }
            "PersonName" => {
                return Some(
                    quote! { helpers::schemasync::coordinate::CoherentDataset::PersonName },
                )
            }
            "GeoLocation" => {
                return Some(
                    quote! { helpers::schemasync::coordinate::CoherentDataset::GeoLocation },
                )
            }
            "DateRange" => {
                return Some(quote! { helpers::schemasync::coordinate::CoherentDataset::DateRange })
            }
            "Financial" => {
                return Some(quote! { helpers::schemasync::coordinate::CoherentDataset::Financial })
            }
            _ => {}
        }
    }
    None
}
