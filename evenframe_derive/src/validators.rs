use helpers::evenframe::validator::{
    ArrayValidator, BigDecimalValidator, BigIntValidator, DateValidator, DurationValidator,
    NumberValidator, StringValidator, Validator,
};
use quote::quote;
use syn::Attribute;

pub fn parse_field_validators(attrs: &[Attribute]) -> Vec<proc_macro2::TokenStream> {
    for attr in attrs {
        if attr.path().is_ident("validators") {
            // Parse the validator expression
            let parse_result = attr.parse_args_with(|input: syn::parse::ParseStream| {
                // Try to parse as a comma-separated list of expressions
                syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_separated_nonempty(
                    input,
                )
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
    let mut validator_tokens = Vec::new();

    // Handle array of validators
    if let syn::Expr::Array(array_expr) = expr {
        for elem in &array_expr.elems {
            validator_tokens.extend(parse_validator_enum(elem));
        }
        return validator_tokens;
    }

    // Handle parenthesized expressions
    if let syn::Expr::Paren(paren) = expr {
        return parse_validator_enum(&paren.expr);
    }

    // Try to parse the expression into a Validator enum using the SynEnum derive
    match Validator::try_from(expr) {
        Ok(validator) => {
            // Match on the actual Validator enum
            let validator_token = match validator {
                Validator::StringValidator(string_validator) => match string_validator {
                    StringValidator::String => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::String) }
                    }
                    StringValidator::Alpha => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Alpha) }
                    }
                    StringValidator::Alphanumeric => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Alphanumeric) }
                    }
                    StringValidator::Base64 => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Base64) }
                    }
                    StringValidator::Base64Url => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Base64Url) }
                    }
                    StringValidator::Capitalize => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Capitalize) }
                    }
                    StringValidator::CapitalizePreformatted => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::CapitalizePreformatted) }
                    }
                    StringValidator::CreditCard => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::CreditCard) }
                    }
                    StringValidator::Date => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Date) }
                    }
                    StringValidator::DateEpoch => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::DateEpoch) }
                    }
                    StringValidator::DateEpochParse => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::DateEpochParse) }
                    }
                    StringValidator::DateIso => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::DateIso) }
                    }
                    StringValidator::DateIsoParse => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::DateIsoParse) }
                    }
                    StringValidator::DateParse => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::DateParse) }
                    }
                    StringValidator::Digits => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Digits) }
                    }
                    StringValidator::Email => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Email) }
                    }
                    StringValidator::Hex => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Hex) }
                    }
                    StringValidator::Integer => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Integer) }
                    }
                    StringValidator::IntegerParse => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::IntegerParse) }
                    }
                    StringValidator::Ip => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Ip) }
                    }
                    StringValidator::IpV4 => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::IpV4) }
                    }
                    StringValidator::IpV6 => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::IpV6) }
                    }
                    StringValidator::Json => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Json) }
                    }
                    StringValidator::JsonParse => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::JsonParse) }
                    }
                    StringValidator::Lower => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Lower) }
                    }
                    StringValidator::LowerPreformatted => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::LowerPreformatted) }
                    }
                    StringValidator::Normalize => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Normalize) }
                    }
                    StringValidator::NormalizeNFC => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::NormalizeNFC) }
                    }
                    StringValidator::NormalizeNFCPreformatted => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::NormalizeNFCPreformatted) }
                    }
                    StringValidator::NormalizeNFD => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::NormalizeNFD) }
                    }
                    StringValidator::NormalizeNFDPreformatted => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::NormalizeNFDPreformatted) }
                    }
                    StringValidator::NormalizeNFKC => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::NormalizeNFKC) }
                    }
                    StringValidator::NormalizeNFKCPreformatted => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::NormalizeNFKCPreformatted) }
                    }
                    StringValidator::NormalizeNFKD => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::NormalizeNFKD) }
                    }
                    StringValidator::NormalizeNFKDPreformatted => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::NormalizeNFKDPreformatted) }
                    }
                    StringValidator::Numeric => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Numeric) }
                    }
                    StringValidator::NumericParse => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::NumericParse) }
                    }
                    StringValidator::Regex => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Regex) }
                    }
                    StringValidator::Semver => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Semver) }
                    }
                    StringValidator::Trim => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Trim) }
                    }
                    StringValidator::TrimPreformatted => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::TrimPreformatted) }
                    }
                    StringValidator::Upper => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Upper) }
                    }
                    StringValidator::UpperPreformatted => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::UpperPreformatted) }
                    }
                    StringValidator::Url => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Url) }
                    }
                    StringValidator::UrlParse => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::UrlParse) }
                    }
                    StringValidator::Uuid => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Uuid) }
                    }
                    StringValidator::UuidV1 => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::UuidV1) }
                    }
                    StringValidator::UuidV2 => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::UuidV2) }
                    }
                    StringValidator::UuidV3 => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::UuidV3) }
                    }
                    StringValidator::UuidV4 => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::UuidV4) }
                    }
                    StringValidator::UuidV5 => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::UuidV5) }
                    }
                    StringValidator::UuidV6 => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::UuidV6) }
                    }
                    StringValidator::UuidV7 => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::UuidV7) }
                    }
                    StringValidator::UuidV8 => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::UuidV8) }
                    }
                    StringValidator::Literal(ref value) => {
                        let value_str = value.clone();
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Literal(#value_str.to_string())) }
                    }
                    StringValidator::StringEmbedded(ref embedded) => {
                        let embedded_str = embedded.clone();
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::StringEmbedded(#embedded_str.to_string())) }
                    }
                    StringValidator::RegexLiteral(ref pattern) => {
                        let pattern_str = pattern.clone();
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::RegexLiteral(#pattern_str.to_string())) }
                    }
                    StringValidator::Length(ref len) => {
                        let len_str = len.clone();
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Length(#len_str.to_string())) }
                    }
                    StringValidator::MinLength(len) => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::MinLength(#len)) }
                    }
                    StringValidator::MaxLength(len) => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::MaxLength(#len)) }
                    }
                    StringValidator::NonEmpty => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::NonEmpty) }
                    }
                    StringValidator::StartsWith(ref prefix) => {
                        let prefix_str = prefix.clone();
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::StartsWith(#prefix_str.to_string())) }
                    }
                    StringValidator::EndsWith(ref suffix) => {
                        let suffix_str = suffix.clone();
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::EndsWith(#suffix_str.to_string())) }
                    }
                    StringValidator::Includes(ref substring) => {
                        let substring_str = substring.clone();
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Includes(#substring_str.to_string())) }
                    }
                    StringValidator::Trimmed => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Trimmed) }
                    }
                    StringValidator::Lowercased => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Lowercased) }
                    }
                    StringValidator::Uppercased => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Uppercased) }
                    }
                    StringValidator::Capitalized => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Capitalized) }
                    }
                    StringValidator::Uncapitalized => {
                        quote! { ::helpers::evenframe::validator::Validator::StringValidator(::helpers::evenframe::validator::StringValidator::Uncapitalized) }
                    }
                },
                Validator::NumberValidator(number_validator) => match number_validator {
                    NumberValidator::GreaterThan(value) => {
                        let float_value = value.0;
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::GreaterThan(::ordered_float::OrderedFloat(#float_value))) }
                    }
                    NumberValidator::GreaterThanOrEqualTo(value) => {
                        let float_value = value.0;
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::GreaterThanOrEqualTo(::ordered_float::OrderedFloat(#float_value))) }
                    }
                    NumberValidator::LessThan(value) => {
                        let float_value = value.0;
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::LessThan(::ordered_float::OrderedFloat(#float_value))) }
                    }
                    NumberValidator::LessThanOrEqualTo(value) => {
                        let float_value = value.0;
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::LessThanOrEqualTo(::ordered_float::OrderedFloat(#float_value))) }
                    }
                    NumberValidator::Between(start, end) => {
                        let start_value = start.0;
                        let end_value = end.0;
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::Between(::ordered_float::OrderedFloat(#start_value), ::ordered_float::OrderedFloat(#end_value))) }
                    }
                    NumberValidator::Int => {
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::Int) }
                    }
                    NumberValidator::NonNaN => {
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::NonNaN) }
                    }
                    NumberValidator::Positive => {
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::Positive) }
                    }
                    NumberValidator::Negative => {
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::Negative) }
                    }
                    NumberValidator::NonPositive => {
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::NonPositive) }
                    }
                    NumberValidator::NonNegative => {
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::NonNegative) }
                    }
                    NumberValidator::Finite => {
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::Finite) }
                    }
                    NumberValidator::MultipleOf(value) => {
                        let float_value = value.0;
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::MultipleOf(::ordered_float::OrderedFloat(#float_value))) }
                    }
                    NumberValidator::Uint8 => {
                        quote! { ::helpers::evenframe::validator::Validator::NumberValidator(::helpers::evenframe::validator::NumberValidator::Uint8) }
                    }
                },
                Validator::ArrayValidator(array_validator) => match array_validator {
                    ArrayValidator::MinItems(count) => {
                        quote! { ::helpers::evenframe::validator::Validator::ArrayValidator(::helpers::evenframe::validator::ArrayValidator::MinItems(#count)) }
                    }
                    ArrayValidator::MaxItems(count) => {
                        quote! { ::helpers::evenframe::validator::Validator::ArrayValidator(::helpers::evenframe::validator::ArrayValidator::MaxItems(#count)) }
                    }
                    ArrayValidator::ItemsCount(count) => {
                        quote! { ::helpers::evenframe::validator::Validator::ArrayValidator(::helpers::evenframe::validator::ArrayValidator::ItemsCount(#count)) }
                    }
                },
                Validator::DateValidator(date_validator) => match date_validator {
                    DateValidator::ValidDate => {
                        quote! { ::helpers::evenframe::validator::Validator::DateValidator(::helpers::evenframe::validator::DateValidator::ValidDate) }
                    }
                    DateValidator::GreaterThanDate(ref date) => {
                        let date_str = date.clone();
                        quote! { ::helpers::evenframe::validator::Validator::DateValidator(::helpers::evenframe::validator::DateValidator::GreaterThanDate(#date_str.to_string())) }
                    }
                    DateValidator::GreaterThanOrEqualToDate(ref date) => {
                        let date_str = date.clone();
                        quote! { ::helpers::evenframe::validator::Validator::DateValidator(::helpers::evenframe::validator::DateValidator::GreaterThanOrEqualToDate(#date_str.to_string())) }
                    }
                    DateValidator::LessThanDate(ref date) => {
                        let date_str = date.clone();
                        quote! { ::helpers::evenframe::validator::Validator::DateValidator(::helpers::evenframe::validator::DateValidator::LessThanDate(#date_str.to_string())) }
                    }
                    DateValidator::LessThanOrEqualToDate(ref date) => {
                        let date_str = date.clone();
                        quote! { ::helpers::evenframe::validator::Validator::DateValidator(::helpers::evenframe::validator::DateValidator::LessThanOrEqualToDate(#date_str.to_string())) }
                    }
                    DateValidator::BetweenDate(ref start, ref end) => {
                        let start_str = start.clone();
                        let end_str = end.clone();
                        quote! { ::helpers::evenframe::validator::Validator::DateValidator(::helpers::evenframe::validator::DateValidator::BetweenDate(#start_str.to_string(), #end_str.to_string())) }
                    }
                },
                Validator::BigIntValidator(bigint_validator) => match bigint_validator {
                    BigIntValidator::GreaterThanBigInt(ref value) => {
                        let value_str = value.clone();
                        quote! { ::helpers::evenframe::validator::Validator::BigIntValidator(::helpers::evenframe::validator::BigIntValidator::GreaterThanBigInt(#value_str.to_string())) }
                    }
                    BigIntValidator::GreaterThanOrEqualToBigInt(ref value) => {
                        let value_str = value.clone();
                        quote! { ::helpers::evenframe::validator::Validator::BigIntValidator(::helpers::evenframe::validator::BigIntValidator::GreaterThanOrEqualToBigInt(#value_str.to_string())) }
                    }
                    BigIntValidator::LessThanBigInt(ref value) => {
                        let value_str = value.clone();
                        quote! { ::helpers::evenframe::validator::Validator::BigIntValidator(::helpers::evenframe::validator::BigIntValidator::LessThanBigInt(#value_str.to_string())) }
                    }
                    BigIntValidator::LessThanOrEqualToBigInt(ref value) => {
                        let value_str = value.clone();
                        quote! { ::helpers::evenframe::validator::Validator::BigIntValidator(::helpers::evenframe::validator::BigIntValidator::LessThanOrEqualToBigInt(#value_str.to_string())) }
                    }
                    BigIntValidator::BetweenBigInt(ref start, ref end) => {
                        let start_str = start.clone();
                        let end_str = end.clone();
                        quote! { ::helpers::evenframe::validator::Validator::BigIntValidator(::helpers::evenframe::validator::BigIntValidator::BetweenBigInt(#start_str.to_string(), #end_str.to_string())) }
                    }
                    BigIntValidator::PositiveBigInt => {
                        quote! { ::helpers::evenframe::validator::Validator::BigIntValidator(::helpers::evenframe::validator::BigIntValidator::PositiveBigInt) }
                    }
                    BigIntValidator::NegativeBigInt => {
                        quote! { ::helpers::evenframe::validator::Validator::BigIntValidator(::helpers::evenframe::validator::BigIntValidator::NegativeBigInt) }
                    }
                    BigIntValidator::NonPositiveBigInt => {
                        quote! { ::helpers::evenframe::validator::Validator::BigIntValidator(::helpers::evenframe::validator::BigIntValidator::NonPositiveBigInt) }
                    }
                    BigIntValidator::NonNegativeBigInt => {
                        quote! { ::helpers::evenframe::validator::Validator::BigIntValidator(::helpers::evenframe::validator::BigIntValidator::NonNegativeBigInt) }
                    }
                },
                Validator::BigDecimalValidator(bigdecimal_validator) => {
                    match bigdecimal_validator {
                        BigDecimalValidator::GreaterThanBigDecimal(ref value) => {
                            let value_str = value.clone();
                            quote! { ::helpers::evenframe::validator::Validator::BigDecimalValidator(::helpers::evenframe::validator::BigDecimalValidator::GreaterThanBigDecimal(#value_str.to_string())) }
                        }
                        BigDecimalValidator::GreaterThanOrEqualToBigDecimal(ref value) => {
                            let value_str = value.clone();
                            quote! { ::helpers::evenframe::validator::Validator::BigDecimalValidator(::helpers::evenframe::validator::BigDecimalValidator::GreaterThanOrEqualToBigDecimal(#value_str.to_string())) }
                        }
                        BigDecimalValidator::LessThanBigDecimal(ref value) => {
                            let value_str = value.clone();
                            quote! { ::helpers::evenframe::validator::Validator::BigDecimalValidator(::helpers::evenframe::validator::BigDecimalValidator::LessThanBigDecimal(#value_str.to_string())) }
                        }
                        BigDecimalValidator::LessThanOrEqualToBigDecimal(ref value) => {
                            let value_str = value.clone();
                            quote! { ::helpers::evenframe::validator::Validator::BigDecimalValidator(::helpers::evenframe::validator::BigDecimalValidator::LessThanOrEqualToBigDecimal(#value_str.to_string())) }
                        }
                        BigDecimalValidator::BetweenBigDecimal(ref start, ref end) => {
                            let start_str = start.clone();
                            let end_str = end.clone();
                            quote! { ::helpers::evenframe::validator::Validator::BigDecimalValidator(::helpers::evenframe::validator::BigDecimalValidator::BetweenBigDecimal(#start_str.to_string(), #end_str.to_string())) }
                        }
                        BigDecimalValidator::PositiveBigDecimal => {
                            quote! { ::helpers::evenframe::validator::Validator::BigDecimalValidator(::helpers::evenframe::validator::BigDecimalValidator::PositiveBigDecimal) }
                        }
                        BigDecimalValidator::NegativeBigDecimal => {
                            quote! { ::helpers::evenframe::validator::Validator::BigDecimalValidator(::helpers::evenframe::validator::BigDecimalValidator::NegativeBigDecimal) }
                        }
                        BigDecimalValidator::NonPositiveBigDecimal => {
                            quote! { ::helpers::evenframe::validator::Validator::BigDecimalValidator(::helpers::evenframe::validator::BigDecimalValidator::NonPositiveBigDecimal) }
                        }
                        BigDecimalValidator::NonNegativeBigDecimal => {
                            quote! { ::helpers::evenframe::validator::Validator::BigDecimalValidator(::helpers::evenframe::validator::BigDecimalValidator::NonNegativeBigDecimal) }
                        }
                    }
                }
                Validator::DurationValidator(duration_validator) => match duration_validator {
                    DurationValidator::GreaterThanDuration(ref value) => {
                        let value_str = value.clone();
                        quote! { ::helpers::evenframe::validator::Validator::DurationValidator(::helpers::evenframe::validator::DurationValidator::GreaterThanDuration(#value_str.to_string())) }
                    }
                    DurationValidator::GreaterThanOrEqualToDuration(ref value) => {
                        let value_str = value.clone();
                        quote! { ::helpers::evenframe::validator::Validator::DurationValidator(::helpers::evenframe::validator::DurationValidator::GreaterThanOrEqualToDuration(#value_str.to_string())) }
                    }
                    DurationValidator::LessThanDuration(ref value) => {
                        let value_str = value.clone();
                        quote! { ::helpers::evenframe::validator::Validator::DurationValidator(::helpers::evenframe::validator::DurationValidator::LessThanDuration(#value_str.to_string())) }
                    }
                    DurationValidator::LessThanOrEqualToDuration(ref value) => {
                        let value_str = value.clone();
                        quote! { ::helpers::evenframe::validator::Validator::DurationValidator(::helpers::evenframe::validator::DurationValidator::LessThanOrEqualToDuration(#value_str.to_string())) }
                    }
                    DurationValidator::BetweenDuration(ref start, ref end) => {
                        let start_str = start.clone();
                        let end_str = end.clone();
                        quote! { ::helpers::evenframe::validator::Validator::DurationValidator(::helpers::evenframe::validator::DurationValidator::BetweenDuration(#start_str.to_string(), #end_str.to_string())) }
                    }
                },
            };
            validator_tokens.push(validator_token);
        }
        Err(_) => {
            // If we can't parse it as a Validator, try to handle it as raw expression
            // This maintains backward compatibility
            panic!("Something went wrong parsing the syn::Expr as a Validator")
        }
    }

    validator_tokens
}
