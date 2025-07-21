use quote::quote;
use syn::Attribute;

pub fn parse_field_validators(attrs: &[Attribute]) -> Vec<proc_macro2::TokenStream> {
    for attr in attrs {
        if attr.path().is_ident("validators") {
            // Parse the validator expression
            let parse_result = attr.parse_args_with(|input: syn::parse::ParseStream| {
                // Try to parse as a comma-separated list of expressions
                syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_separated_nonempty(input)
            });
            
            match parse_result {
                Ok(validators_list) => {
                    let mut validators = Vec::new();
                    for validator_expr in validators_list {
                        validators.extend(parse_validator_enum(&validator_expr));
                    }
                    return validators;
                }
                Err(_) => {
                    // Try parsing as a single expression for backwards compatibility
                    match attr.parse_args::<syn::Expr>() {
                        Ok(expr) => return parse_validator_enum(&expr),
                        Err(_) => continue,
                    }
                }
            }
        }
    }
    vec![]
}

// Parse a validator enum expression
pub fn parse_validator_enum(expr: &syn::Expr) -> Vec<proc_macro2::TokenStream> {
    let mut validators = Vec::new();
    
    match expr {
        // Handle direct enum paths like StringValidator::Email
        syn::Expr::Path(path_expr) => {
            let path = &path_expr.path;
            
            // Check if it's a direct enum variant (e.g., StringValidator::Email)
            if path.segments.len() >= 2 {
                let enum_name = &path.segments[path.segments.len() - 2].ident;
                let variant_name = &path.segments[path.segments.len() - 1].ident;
                
                match enum_name.to_string().as_str() {
                    "StringValidator" => {
                        validators.push(quote! {
                            ::helpers::evenframe::validator::Validator::StringValidator(
                                ::helpers::evenframe::validator::StringValidator::#variant_name
                            )
                        });
                    }
                    "NumberValidator" => {
                        validators.push(quote! {
                            ::helpers::evenframe::validator::Validator::NumberValidator(
                                ::helpers::evenframe::validator::NumberValidator::#variant_name
                            )
                        });
                    }
                    "ArrayValidator" => {
                        validators.push(quote! {
                            ::helpers::evenframe::validator::Validator::ArrayValidator(
                                ::helpers::evenframe::validator::ArrayValidator::#variant_name
                            )
                        });
                    }
                    "DateValidator" => {
                        validators.push(quote! {
                            ::helpers::evenframe::validator::Validator::DateValidator(
                                ::helpers::evenframe::validator::DateValidator::#variant_name
                            )
                        });
                    }
                    "BigIntValidator" => {
                        validators.push(quote! {
                            ::helpers::evenframe::validator::Validator::BigIntValidator(
                                ::helpers::evenframe::validator::BigIntValidator::#variant_name
                            )
                        });
                    }
                    "BigDecimalValidator" => {
                        validators.push(quote! {
                            ::helpers::evenframe::validator::Validator::BigDecimalValidator(
                                ::helpers::evenframe::validator::BigDecimalValidator::#variant_name
                            )
                        });
                    }
                    "DurationValidator" => {
                        validators.push(quote! {
                            ::helpers::evenframe::validator::Validator::DurationValidator(
                                ::helpers::evenframe::validator::DurationValidator::#variant_name
                            )
                        });
                    }
                    _ => {}
                }
            } else if path.segments.len() == 1 {
                // Handle shorthand variants when inside validator module context
                let variant_name = &path.segments[0].ident;
                
                // Try to infer the validator type based on common variant names
                match variant_name.to_string().as_str() {
                    // Common string validators
                    "Email" | "Url" | "Uuid" | "Alpha" | "Alphanumeric" | "Digits" |
                    "CreditCard" | "Semver" | "Ip" | "Ipv4" | "Ipv6" | "Mac" | "Jwt" |
                    "DateIso" | "Ulid" | "Cuid" | "Cuid2" | "Nanoid" | "Duration" |
                    "Trimmed" | "Lowercased" | "Uppercased" | "Capitalized" |
                    "Trim" | "Lowercase" | "Uppercase" | "Capitalize" | "NonEmpty" |
                    "UuidV1" | "UuidV3" | "UuidV4" | "UuidV5" | "UuidV6" | "UuidV7" | "UuidV8" |
                    "NormalizeNfc" | "NormalizeNfd" | "NormalizeNfkc" | "NormalizeNfkd" => {
                        validators.push(quote! {
                            ::helpers::evenframe::validator::Validator::StringValidator(
                                ::helpers::evenframe::validator::StringValidator::#variant_name
                            )
                        });
                    }
                    
                    // Common number validators  
                    "Positive" | "Negative" | "NonPositive" | "NonNegative" |
                    "Finite" | "Safe" | "Int" => {
                        validators.push(quote! {
                            ::helpers::evenframe::validator::Validator::NumberValidator(
                                ::helpers::evenframe::validator::NumberValidator::#variant_name
                            )
                        });
                    }
                    
                    // Common array validators
                    "NoEmpty" => {
                        validators.push(quote! {
                            ::helpers::evenframe::validator::Validator::ArrayValidator(
                                ::helpers::evenframe::validator::ArrayValidator::#variant_name
                            )
                        });
                    }
                    
                    _ => {}
                }
            }
        }
        
        // Handle enum variant calls with parameters like StringValidator::MinLength(5)
        syn::Expr::Call(call_expr) => {
            if let syn::Expr::Path(path_expr) = &*call_expr.func {
                let path = &path_expr.path;
                
                if path.segments.len() >= 2 {
                    let enum_name = &path.segments[path.segments.len() - 2].ident;
                    let variant_name = &path.segments[path.segments.len() - 1].ident;
                    
                    match enum_name.to_string().as_str() {
                        "StringValidator" => {
                            // Handle string validators with parameters
                            match variant_name.to_string().as_str() {
                                "MinLength" | "MaxLength" => {
                                    if let Some(arg) = call_expr.args.first() {
                                        validators.push(quote! {
                                            ::helpers::evenframe::validator::Validator::StringValidator(
                                                ::helpers::evenframe::validator::StringValidator::#variant_name(#arg)
                                            )
                                        });
                                    }
                                }
                                "Length" | "RegexLiteral" | "StartsWith" | "EndsWith" | 
                                "Includes" | "Excludes" | "Pattern" | "StringEmbedded" => {
                                    if let Some(arg) = call_expr.args.first() {
                                        validators.push(quote! {
                                            ::helpers::evenframe::validator::Validator::StringValidator(
                                                ::helpers::evenframe::validator::StringValidator::#variant_name(#arg.to_string())
                                            )
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                        "NumberValidator" => {
                            // Handle number validators with parameters
                            match variant_name.to_string().as_str() {
                                "Min" | "Max" | "LessThan" | "GreaterThan" | "MultipleOf" => {
                                    if let Some(arg) = call_expr.args.first() {
                                        validators.push(quote! {
                                            ::helpers::evenframe::validator::Validator::NumberValidator(
                                                ::helpers::evenframe::validator::NumberValidator::#variant_name(#arg)
                                            )
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                        "ArrayValidator" => {
                            // Handle array validators with parameters
                            match variant_name.to_string().as_str() {
                                "MinItems" | "MaxItems" => {
                                    if let Some(arg) = call_expr.args.first() {
                                        validators.push(quote! {
                                            ::helpers::evenframe::validator::Validator::ArrayValidator(
                                                ::helpers::evenframe::validator::ArrayValidator::#variant_name(#arg)
                                            )
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                        "DateValidator" => {
                            // Handle date validators with parameters
                            match variant_name.to_string().as_str() {
                                "MinDate" | "MaxDate" => {
                                    if let Some(arg) = call_expr.args.first() {
                                        validators.push(quote! {
                                            ::helpers::evenframe::validator::Validator::DateValidator(
                                                ::helpers::evenframe::validator::DateValidator::#variant_name(#arg)
                                            )
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                        "BigIntValidator" => {
                            // Handle bigint validators with parameters
                            match variant_name.to_string().as_str() {
                                "Min" | "Max" | "LessThan" | "GreaterThan" | "MultipleOf" => {
                                    if let Some(arg) = call_expr.args.first() {
                                        validators.push(quote! {
                                            ::helpers::evenframe::validator::Validator::BigIntValidator(
                                                ::helpers::evenframe::validator::BigIntValidator::#variant_name(#arg)
                                            )
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                        "BigDecimalValidator" => {
                            // Handle bigdecimal validators with parameters
                            match variant_name.to_string().as_str() {
                                "Min" | "Max" | "LessThan" | "GreaterThan" | "MultipleOf" => {
                                    if let Some(arg) = call_expr.args.first() {
                                        validators.push(quote! {
                                            ::helpers::evenframe::validator::Validator::BigDecimalValidator(
                                                ::helpers::evenframe::validator::BigDecimalValidator::#variant_name(#arg)
                                            )
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                        "DurationValidator" => {
                            // Handle duration validators with parameters
                            match variant_name.to_string().as_str() {
                                "Min" | "Max" => {
                                    if let Some(arg) = call_expr.args.first() {
                                        validators.push(quote! {
                                            ::helpers::evenframe::validator::Validator::DurationValidator(
                                                ::helpers::evenframe::validator::DurationValidator::#variant_name(#arg)
                                            )
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // Handle shorthand constructor calls when using imports
        syn::Expr::MethodCall(method_call) => {
            // This would handle cases like Email() if Email was imported
            if let syn::Expr::Path(path_expr) = &*method_call.receiver {
                if let Some(ident) = path_expr.path.get_ident() {
                    let variant_name = ident;
                    
                    // Try to infer based on common names
                    match ident.to_string().as_str() {
                        "Email" | "Url" | "Uuid" | "Alpha" | "Alphanumeric" => {
                            validators.push(quote! {
                                ::helpers::evenframe::validator::Validator::StringValidator(
                                    ::helpers::evenframe::validator::StringValidator::#variant_name
                                )
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // Handle array of validators
        syn::Expr::Array(array_expr) => {
            for elem in &array_expr.elems {
                validators.extend(parse_validator_enum(elem));
            }
        }
        
        // Handle parenthesized expressions
        syn::Expr::Paren(paren) => {
            validators.extend(parse_validator_enum(&paren.expr));
        }
        
        _ => {}
    }
    
    validators
}