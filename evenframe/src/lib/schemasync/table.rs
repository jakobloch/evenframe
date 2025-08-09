use crate::config::EvenframeConfig;
use crate::mockmake::MockGenerationConfig;
use crate::schemasync::edge::EdgeConfig;
use crate::schemasync::permissions::PermissionsConfig;
use crate::schemasync::surql::define::generate_define_statements;
use crate::types::StructConfig;
use crate::validator::{ArrayValidator, NumberValidator, StringValidator, Validator};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TableConfig {
    pub struct_config: StructConfig,
    pub relation: Option<EdgeConfig>,
    pub permissions: Option<PermissionsConfig>,
    pub mock_generation_config: Option<MockGenerationConfig>,
}

impl TableConfig {
    /// Generate DEFINE statements for this table as a vector of strings
    pub fn generate_define_statements(&self, table_name: &str) -> Vec<String> {
        // For now, return a single string wrapped in a vector
        // This maintains compatibility with the merge module's expectation
        vec![self.generate_define_statements_string(table_name)]
    }

    /// Generate DEFINE statements for this table as a single string
    pub fn generate_define_statements_string(&self, table_name: &str) -> String {
        let evenframe_config =
            EvenframeConfig::new().expect("Something went wrong getting the evenframe config");
        // We need empty collections since this is just for schema generation
        let empty_tables = HashMap::new();
        let empty_objects = HashMap::new();
        let empty_enums = HashMap::new();

        // Call the standalone function
        generate_define_statements(
            table_name,
            self,
            &empty_tables,
            &empty_objects,
            &empty_enums,
            evenframe_config
                .schemasync
                .mock_gen_config
                .full_refresh_mode,
        )
    }
}

/// Generate ASSERT clause from validators
pub fn generate_assert_from_validators(validators: &[Validator], value_var: &str) -> String {
    let mut assertions = Vec::new();

    for validator in validators {
        match validator {
            // String validators
            Validator::StringValidator(sv) => match sv {
                StringValidator::Email => {
                    assertions.push(format!("string::is::email({})", value_var))
                }
                StringValidator::Alpha => {
                    assertions.push(format!("string::is::alpha({})", value_var))
                }
                StringValidator::Alphanumeric => {
                    assertions.push(format!("string::is::alphanum({})", value_var))
                }
                StringValidator::Hex => {
                    assertions.push(format!("string::is::hexadecimal({})", value_var))
                }
                StringValidator::Ip => assertions.push(format!("string::is::ip({})", value_var)),
                StringValidator::IpV4 => {
                    assertions.push(format!("string::is::ipv4({})", value_var))
                }
                StringValidator::IpV6 => {
                    assertions.push(format!("string::is::ipv6({})", value_var))
                }
                StringValidator::Url => assertions.push(format!("string::is::url({})", value_var)),
                StringValidator::Uuid => {
                    assertions.push(format!("string::is::uuid({})", value_var))
                }
                StringValidator::Semver => {
                    assertions.push(format!("string::is::semver({})", value_var))
                }
                StringValidator::Digits => {
                    assertions.push(format!("string::is::numeric({})", value_var))
                }
                StringValidator::MinLength(len) => {
                    assertions.push(format!("string::len({}) >= {}", value_var, len))
                }
                StringValidator::MaxLength(len) => {
                    assertions.push(format!("string::len({}) <= {}", value_var, len))
                }
                StringValidator::Length(len) => {
                    assertions.push(format!("string::len({}) = {}", value_var, len))
                }
                StringValidator::NonEmpty => {
                    assertions.push(format!("string::len({}) > 0", value_var))
                }
                StringValidator::StartsWith(prefix) => assertions.push(format!(
                    "string::starts_with({}, \"{}\")",
                    value_var, prefix
                )),
                StringValidator::EndsWith(suffix) => {
                    assertions.push(format!("string::ends_with({}, \"{}\")", value_var, suffix))
                }
                StringValidator::Includes(substring) => assertions.push(format!(
                    "string::contains({}, \"{}\")",
                    value_var, substring
                )),
                StringValidator::Trimmed => {
                    assertions.push(format!("{} = string::trim({})", value_var, value_var))
                }
                StringValidator::Lowercased => {
                    assertions.push(format!("{} = string::lowercase({})", value_var, value_var))
                }
                StringValidator::Uppercased => {
                    assertions.push(format!("{} = string::uppercase({})", value_var, value_var))
                }
                StringValidator::RegexLiteral(format_variant) => assertions.push(format!(
                    "string::matches({}, \"{}\")",
                    value_var,
                    format_variant.to_owned().into_regex().as_str()
                )),
                _ => {} // Other validators don't have direct SurrealDB equivalents
            },

            // Number validators
            Validator::NumberValidator(nv) => match nv {
                NumberValidator::GreaterThan(value) => {
                    assertions.push(format!("{} > {}", value_var, value.0))
                }
                NumberValidator::GreaterThanOrEqualTo(value) => {
                    assertions.push(format!("{} >= {}", value_var, value.0))
                }
                NumberValidator::LessThan(value) => {
                    assertions.push(format!("{} < {}", value_var, value.0))
                }
                NumberValidator::LessThanOrEqualTo(value) => {
                    assertions.push(format!("{} <= {}", value_var, value.0))
                }
                NumberValidator::Between(start, end) => assertions.push(format!(
                    "{} >= {} AND {} <= {}",
                    value_var, start.0, value_var, end.0
                )),
                NumberValidator::Int => assertions.push(format!("type::is::int({})", value_var)),
                NumberValidator::Positive => assertions.push(format!("{} > 0", value_var)),
                NumberValidator::NonNegative => assertions.push(format!("{} >= 0", value_var)),
                NumberValidator::Negative => assertions.push(format!("{} < 0", value_var)),
                NumberValidator::NonPositive => assertions.push(format!("{} <= 0", value_var)),
                NumberValidator::MultipleOf(value) => {
                    assertions.push(format!("{} % {} = 0", value_var, value.0))
                }
                _ => {} // NonNaN, Finite, Uint8 don't have direct equivalents
            },

            // Array validators
            Validator::ArrayValidator(av) => match av {
                ArrayValidator::MinItems(count) => {
                    assertions.push(format!("array::len({}) >= {}", value_var, count))
                }
                ArrayValidator::MaxItems(count) => {
                    assertions.push(format!("array::len({}) <= {}", value_var, count))
                }
                ArrayValidator::ItemsCount(count) => {
                    assertions.push(format!("array::len({}) = {}", value_var, count))
                }
            },

            // Date validators - SurrealDB doesn't have built-in date comparison functions
            // These would need custom functions or direct comparisons
            Validator::DateValidator(_) => {}

            // BigInt validators - SurrealDB doesn't have separate BigInt type
            Validator::BigIntValidator(_) => {}

            // BigDecimal validators - SurrealDB doesn't have BigDecimal type
            Validator::BigDecimalValidator(_) => {}

            // Duration validators - Would need custom functions
            Validator::DurationValidator(_) => {}
        }
    }

    // Join all assertions with AND
    if assertions.is_empty() {
        String::new()
    } else {
        assertions.join(" AND ")
    }
}
