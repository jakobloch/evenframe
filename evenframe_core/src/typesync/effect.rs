use crate::dependency::{RecursionInfo, analyse_recursion, deps_of};
use crate::types::{FieldType, StructConfig, TaggedUnion, VariantData};
use crate::validator::{
    ArrayValidator, BigDecimalValidator, BigIntValidator, DateValidator, DurationValidator,
    NumberValidator, StringValidator, Validator,
};
use convert_case::{Case, Casing};
use petgraph::{algo::toposort, graphmap::DiGraphMap};
use std::collections::{HashMap, HashSet};
use tracing;

pub fn generate_effect_schema_string(
    structs: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
    print_types: bool,
) -> String {
    tracing::info!(
        struct_count = structs.len(),
        enum_count = enums.len(),
        print_types = print_types,
        "Generating Effect Schema string"
    );

    // 1.  Analyse recursion once at the beginning.
    tracing::debug!("Analyzing recursion in types");
    let rec = analyse_recursion(structs, enums);

    // 2.  Topologically sort components so all **non-recursive**
    //     dependencies appear first. This removes the need for
    //     `Schema.suspend` outside of recursive strongly connected components (SCCs).
    tracing::debug!("Performing topological sort of components");
    let mut condensation = DiGraphMap::<usize, ()>::new();
    for (t1, _tos) in rec
        .meta
        .values()
        .flat_map(|(_, mem)| mem.iter())
        .filter_map(|n| rec.comp_of.get(n).map(|&c| (n, c)))
    {
        let from_comp = rec.comp_of[t1];
        for t2 in &deps_of(t1, structs, enums) {
            let to_comp = rec.comp_of[t2];
            if from_comp != to_comp {
                // An edge A -> B means "A depends on B".
                condensation.add_edge(from_comp, to_comp, ());
            }
        }
    }
    // `toposort` gives an order where dependencies come first. We reverse it
    // to process dependencies before the types that use them.
    let mut ordered_comps = toposort(&condensation, None).unwrap_or_default();
    ordered_comps.reverse();

    // 3.  Generate all TypeScript code in a single, unified loop.
    tracing::debug!("Generating schema classes, types, and encoded interfaces");
    let mut out_classes = String::new();
    let mut out_types = String::new();
    let mut out_encoded = String::new(); // All '...Encoded' interfaces/types go here.
    let mut processed = HashSet::<String>::new();

    // Helper closure for field conversion that has access to `rec`.
    let to_schema = |ft: &FieldType, cur: &str, proc: &HashSet<String>| -> String {
        field_type_to_effect_schema(ft, structs, cur, &rec, proc)
    };

    for comp_id in ordered_comps {
        // Order inside the SCC is arbitrary; preserve original order for deterministic output.
        let mut members = rec.meta[&comp_id].1.clone();
        members.sort();

        for name in members {
            if processed.contains(&name) {
                continue; // Skip if already processed
            }

            if let Some(e) = enums
                .values()
                .find(|e| e.enum_name.to_case(Case::Pascal) == name)
            {
                // ---- ENUM ---------------------------------------------------
                // Generate the schema class for the enum.
                out_classes.push_str(&format!("export const {} = Schema.Union(", name));
                let variants = e
                    .variants
                    .iter()
                    .map(|v| {
                        v.data
                            .as_ref()
                            .map(|variant_data| match variant_data {
                                VariantData::InlineStruct(_) => v.name.to_case(Case::Pascal),
                                VariantData::DataStructureRef(field_type) => {
                                    to_schema(field_type, &name, &processed)
                                }
                            })
                            .unwrap_or_else(|| format!("Schema.Literal(\"{}\")", v.name))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                out_classes.push_str(&variants);
                out_classes.push_str(&format!(").annotations({{ identifier: `{}` }});\n", name));

                // Generate the `.Type` alias.
                out_types.push_str(&format!(
                    "export type {}Type = typeof {}.Type;\n",
                    name, name
                ));

                // Generate the `...Encoded` type alias for the enum.
                out_encoded.push_str(&encoded_alias_for_enum(e));
            } else if let Some(struct_config) = structs
                .values()
                .find(|sc| sc.struct_name.to_case(Case::Pascal) == name)
            {
                // ---- STRUCT -------------------------------------------------
                // Generate the schema class for the struct.
                out_classes.push_str(&format!(
                    "export class {} extends Schema.Class<{}>(\"{}\")( {{\n",
                    name, name, name
                ));
                for (idx, f) in struct_config.fields.iter().enumerate() {
                    let schema = to_schema(&f.field_type, &name, &processed);
                    let schema_with_validators = apply_validators_to_schema(schema, &f.validators);
                    out_classes.push_str(&format!(
                        "  {}: {}{}",
                        f.field_name.to_case(Case::Camel),
                        schema_with_validators,
                        if idx + 1 == struct_config.fields.len() {
                            ""
                        } else {
                            ","
                        }
                    ));
                    out_classes.push('\n');
                }
                out_classes.push_str("}) {[key: string]: unknown}\n\n");

                // Generate the `.Type` alias.
                out_types.push_str(&format!(
                    "export type {}Type = typeof {}.Type;\n",
                    name, name
                ));

                // Generate the `...Encoded` interface for the struct.
                out_encoded.push_str(&encoded_interface_for_struct(struct_config));
            }
            processed.insert(name);
        }
    }

    let result = if print_types {
        format!("{out_classes}\n{out_encoded}\n{out_types}")
    } else {
        format!("{out_classes}\n{out_encoded}")
    };

    tracing::info!(
        output_length = result.len(),
        "Effect Schema generation complete"
    );
    result
}

// ----- Encoded Type Generation Helpers -------------------------------------

/// Generates an `...Encoded` TypeScript interface for a given struct.
fn encoded_interface_for_struct(struct_config: &StructConfig) -> String {
    let name = struct_config.struct_name.to_case(Case::Pascal);
    let body = struct_config
        .fields
        .iter()
        .map(|f| {
            format!(
                "  readonly {}: {};",
                f.field_name.to_case(Case::Camel),
                field_type_to_ts_encoded(&f.field_type)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("export interface {}Encoded {{\n{}\n}}\n\n", name, body)
}

/// Generates an `...Encoded` TypeScript type alias for a given enum/union.
fn encoded_alias_for_enum(en: &TaggedUnion) -> String {
    tracing::trace!(enum_name = %en.enum_name, "Creating encoded alias for enum");
    let name = en.enum_name.to_case(Case::Pascal);
    let union = en
        .variants
        .iter()
        .map(|v| match &v.data {
            Some(variant_data) => match variant_data {
                VariantData::InlineStruct(_) => {
                    // For inline structs, use the variant name + "Encoded"
                    format!("{}Encoded", v.name.to_case(Case::Pascal))
                }
                VariantData::DataStructureRef(field_type) => field_type_to_ts_encoded(field_type),
            },
            None => format!("\"{}\"", v.name),
        })
        .collect::<Vec<_>>()
        .join(" | ");

    format!("export type {}Encoded = {};\n\n", name, union)
}

// ----- Schema and Type Conversion Logic ------------------------------------

/// Converts a `FieldType` into its corresponding Effect `Schema` representation.
fn field_type_to_effect_schema(
    field_type: &FieldType,
    structs: &HashMap<String, StructConfig>,
    current: &str,
    rec: &RecursionInfo,
    processed: &HashSet<String>,
) -> String {
    // Helper to recurse with the same context.
    let field = |inner: &FieldType| -> String {
        field_type_to_effect_schema(inner, structs, current, rec, processed)
    };
    match field_type {
        FieldType::String => "Schema.String".to_string(),
        FieldType::Char => "Schema.String".to_string(),
        FieldType::Bool => "Schema.Boolean".to_string(),
        FieldType::Unit => "Schema.Null".to_string(),
        FieldType::Decimal => "Schema.Number".to_string(),
        FieldType::OrderedFloat(_) => "Schema.Number".to_string(),
        FieldType::F32 | FieldType::F64 => "Schema.Number".to_string(),
        FieldType::I8
        | FieldType::I16
        | FieldType::I32
        | FieldType::I64
        | FieldType::I128
        | FieldType::Isize => "Schema.Number".to_string(),
        FieldType::U8
        | FieldType::U16
        | FieldType::U32
        | FieldType::U64
        | FieldType::U128
        | FieldType::Usize => "Schema.Number".to_string(),
        FieldType::EvenframeRecordId => "Schema.String".to_string(),
        FieldType::DateTime => "Schema.Date".to_string(), // Changed from DateTimeUtc for better compatibility
        FieldType::EvenframeDuration => "Schema.Duration".to_string(),
        FieldType::Timezone => "Schema.String".to_string(), // IANA timezone string
        FieldType::Option(i) => format!("Schema.OptionFromNullishOr({}, null)", field(i)),
        FieldType::Vec(i) => format!("Schema.Array({})", field(i)),
        FieldType::Tuple(v) => format!(
            "Schema.Tuple({})",
            v.iter().map(field).collect::<Vec<_>>().join(", ")
        ),
        FieldType::Struct(fs) => {
            let inner = fs
                .iter()
                .map(|(n, f)| format!("{}: {}", n, field(f)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Schema.Struct({{ {} }})", inner)
        }
        FieldType::RecordLink(i) => format!("Schema.Union(Schema.String, {})", field(i)),
        FieldType::HashMap(k, v) | FieldType::BTreeMap(k, v) => {
            format!(
                "Schema.Record({{ key: {}, value: {} }})",
                field(k),
                field(v)
            )
        }
        FieldType::Other(name) => {
            let pascal = name.to_case(Case::Pascal);
            let wrap_id = format!("{}Ref", pascal);
            // Decide whether we need Schema.suspend for recursion.
            if rec.is_recursive_pair(current, &pascal) && !processed.contains(&pascal) {
                // Forward edge *inside* a recursive SCC requires suspension.
                if structs
                    .values()
                    .any(|sc| sc.struct_name.to_case(Case::Pascal) == pascal)
                {
                    format!(
                        "Schema.suspend((): Schema.Schema<{}, {}Encoded> => {}).annotations({{ identifier: `{}` }})",
                        pascal, pascal, pascal, wrap_id
                    )
                } else {
                    format!(
                        "Schema.suspend((): Schema.Schema<typeof {}.Type, {}Encoded> => {}).annotations({{ identifier: `{}` }})",
                        pascal, pascal, pascal, wrap_id
                    )
                }
            } else {
                // Direct reference for non-recursive or already processed types.
                pascal
            }
        }
    }
}

/// Converts a `FieldType` into its corresponding raw TypeScript type for the `...Encoded` interface.
fn field_type_to_ts_encoded(ft: &FieldType) -> String {
    let enc = |f: &FieldType| field_type_to_ts_encoded(f);

    match ft {
        // Primitives
        FieldType::String
        | FieldType::Char
        | FieldType::EvenframeRecordId
        | FieldType::Timezone => "string".into(),
        FieldType::Bool => "boolean".into(),
        FieldType::DateTime => "string".into(), // ISO 8601 string
        FieldType::EvenframeDuration => "Schema.DurationEncoded".into(),
        FieldType::Unit => "null".into(),
        FieldType::Decimal
        | FieldType::OrderedFloat(_)
        | FieldType::F32
        | FieldType::F64
        | FieldType::I8
        | FieldType::I16
        | FieldType::I32
        | FieldType::I64
        | FieldType::I128
        | FieldType::Isize
        | FieldType::U8
        | FieldType::U16
        | FieldType::U32
        | FieldType::U64
        | FieldType::U128
        | FieldType::Usize => "number".into(),

        // Containers
        FieldType::Option(inner) => format!("{} | null | undefined", enc(inner)),
        FieldType::Vec(inner) => format!("ReadonlyArray<{}>", enc(inner)),
        FieldType::Tuple(items) => {
            let elems = items.iter().map(enc).collect::<Vec<_>>().join(", ");
            format!("[{}]", elems)
        }
        FieldType::Struct(fs) => {
            let body = fs
                .iter()
                .map(|(n, f)| format!("  readonly {}: {};", n, enc(f)))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{{\n{}\n}}", body)
        }
        FieldType::HashMap(k, v) | FieldType::BTreeMap(k, v) => {
            format!("Record<{}, {}>", enc(k), enc(v))
        }
        FieldType::RecordLink(inner) => format!("string | {}", enc(inner)),

        // User-defined types
        FieldType::Other(name) => format!("{}Encoded", name.to_case(Case::Pascal)),
    }
}

// ----- Validator Application Logic -----------------------------------------

/// Applies a series of validators to a schema string by chaining `.pipe()` calls.
fn apply_validators_to_schema(schema: String, validators: &[Validator]) -> String {
    if validators.is_empty() {
        return schema;
    }

    let mut result = schema;

    for validator in validators {
        result = match validator {
            // String validators
            Validator::StringValidator(sv) => match sv {
                StringValidator::MinLength(len) => {
                    format!("{}.pipe(Schema.minLength({}))", result, len)
                }
                StringValidator::MaxLength(len) => {
                    format!("{}.pipe(Schema.maxLength({}))", result, len)
                }
                StringValidator::Length(len) => format!("{}.pipe(Schema.length({}))", result, len),
                StringValidator::NonEmpty => format!("{}.pipe(Schema.nonEmpty())", result), // Corrected from nonEmptyString
                StringValidator::StartsWith(prefix) => {
                    format!("{}.pipe(Schema.startsWith(\"{}\"))", result, prefix)
                }
                StringValidator::EndsWith(suffix) => {
                    format!("{}.pipe(Schema.endsWith(\"{}\"))", result, suffix)
                }
                StringValidator::Includes(substring) => {
                    format!("{}.pipe(Schema.includes(\"{}\"))", result, substring)
                }
                StringValidator::Trimmed => format!("{}.pipe(Schema.trimmed)", result), // Corrected to be property access
                StringValidator::Lowercased => format!("{}.pipe(Schema.toLowerCase)", result), // Corrected to be property access
                StringValidator::Uppercased => format!("{}.pipe(Schema.toUpperCase)", result), // Corrected to be property access
                StringValidator::Capitalized => format!("{}.pipe(Schema.capitalize)", result), // Corrected to be property access
                StringValidator::Uncapitalized => format!("{}.pipe(Schema.uncapitalize)", result), // Corrected to be property access
                StringValidator::RegexLiteral(format_variant) => format!(
                    "{}.pipe(Schema.pattern(/{}/))",
                    result,
                    format_variant.to_owned().into_regex().as_str()
                ),
                _ => result, // Other validators may not have direct Effect Schema equivalents
            },

            // Number validators
            Validator::NumberValidator(nv) => match nv {
                NumberValidator::GreaterThan(value) => {
                    format!("{}.pipe(Schema.greaterThan({}))", result, value.0)
                }
                NumberValidator::GreaterThanOrEqualTo(value) => {
                    format!("{}.pipe(Schema.greaterThanOrEqualTo({}))", result, value.0)
                }
                NumberValidator::LessThan(value) => {
                    format!("{}.pipe(Schema.lessThan({}))", result, value.0)
                }
                NumberValidator::LessThanOrEqualTo(value) => {
                    format!("{}.pipe(Schema.lessThanOrEqualTo({}))", result, value.0)
                }
                NumberValidator::Between(start, end) => {
                    format!("{}.pipe(Schema.between({}, {}))", result, start.0, end.0)
                }
                NumberValidator::Int => format!("{}.pipe(Schema.int())", result),
                NumberValidator::NonNaN => format!("{}.pipe(Schema.nonNaN())", result),
                NumberValidator::Finite => format!("{}.pipe(Schema.finite())", result),
                NumberValidator::Positive => format!("{}.pipe(Schema.positive())", result),
                NumberValidator::NonNegative => format!("{}.pipe(Schema.nonNegative())", result),
                NumberValidator::Negative => format!("{}.pipe(Schema.negative())", result),
                NumberValidator::NonPositive => format!("{}.pipe(Schema.nonPositive())", result),
                NumberValidator::MultipleOf(value) => {
                    format!("{}.pipe(Schema.multipleOf({}))", result, value.0)
                }
                NumberValidator::Uint8 => result, // Schema.Uint8 should be used as base type instead
            },

            // Array validators
            Validator::ArrayValidator(av) => match av {
                ArrayValidator::MinItems(count) => {
                    format!("{}.pipe(Schema.minItems({}))", result, count)
                }
                ArrayValidator::MaxItems(count) => {
                    format!("{}.pipe(Schema.maxItems({}))", result, count)
                }
                ArrayValidator::ItemsCount(count) => {
                    format!("{}.pipe(Schema.itemsCount({}))", result, count)
                }
            },

            // Date validators
            Validator::DateValidator(dv) => match dv {
                DateValidator::ValidDate => format!("{}.pipe(Schema.ValidDate)", result), // Assuming this exists
                DateValidator::GreaterThanDate(date) => format!(
                    "{}.pipe(Schema.greaterThan(new Date(\"{}\")))",
                    result, date
                ),
                DateValidator::GreaterThanOrEqualToDate(date) => format!(
                    "{}.pipe(Schema.greaterThanOrEqualTo(new Date(\"{}\")))",
                    result, date
                ),
                DateValidator::LessThanDate(date) => {
                    format!("{}.pipe(Schema.lessThan(new Date(\"{}\")))", result, date)
                }
                DateValidator::LessThanOrEqualToDate(date) => format!(
                    "{}.pipe(Schema.lessThanOrEqualTo(new Date(\"{}\")))",
                    result, date
                ),
                DateValidator::BetweenDate(start, end) => format!(
                    "{}.pipe(Schema.between(new Date(\"{}\"), new Date(\"{}\")))",
                    result, start, end
                ),
            },

            // BigInt validators
            Validator::BigIntValidator(biv) => match biv {
                BigIntValidator::GreaterThanBigInt(value) => {
                    format!("{}.pipe(Schema.greaterThanBigInt({}n))", result, value)
                }
                BigIntValidator::GreaterThanOrEqualToBigInt(value) => format!(
                    "{}.pipe(Schema.greaterThanOrEqualToBigInt({}n))",
                    result, value
                ),
                BigIntValidator::LessThanBigInt(value) => {
                    format!("{}.pipe(Schema.lessThanBigInt({}n))", result, value)
                }
                BigIntValidator::LessThanOrEqualToBigInt(value) => format!(
                    "{}.pipe(Schema.lessThanOrEqualToBigInt({}n))",
                    result, value
                ),
                BigIntValidator::BetweenBigInt(start, end) => format!(
                    "{}.pipe(Schema.betweenBigInt({}n, {}n))",
                    result, start, end
                ),
                BigIntValidator::PositiveBigInt => {
                    format!("{}.pipe(Schema.positiveBigInt())", result)
                }
                BigIntValidator::NonNegativeBigInt => {
                    format!("{}.pipe(Schema.nonNegativeBigInt())", result)
                }
                BigIntValidator::NegativeBigInt => {
                    format!("{}.pipe(Schema.negativeBigInt())", result)
                }
                BigIntValidator::NonPositiveBigInt => {
                    format!("{}.pipe(Schema.nonPositiveBigInt())", result)
                }
            },

            // BigDecimal validators
            Validator::BigDecimalValidator(bdv) => match bdv {
                BigDecimalValidator::GreaterThanBigDecimal(value) => format!(
                    "{}.pipe(Schema.greaterThanBigDecimal(BigDecimal.fromNumber({})))",
                    result, value
                ),
                BigDecimalValidator::GreaterThanOrEqualToBigDecimal(value) => format!(
                    "{}.pipe(Schema.greaterThanOrEqualToBigDecimal(BigDecimal.fromNumber({})))",
                    result, value
                ),
                BigDecimalValidator::LessThanBigDecimal(value) => format!(
                    "{}.pipe(Schema.lessThanBigDecimal(BigDecimal.fromNumber({})))",
                    result, value
                ),
                BigDecimalValidator::LessThanOrEqualToBigDecimal(value) => format!(
                    "{}.pipe(Schema.lessThanOrEqualToBigDecimal(BigDecimal.fromNumber({})))",
                    result, value
                ),
                BigDecimalValidator::BetweenBigDecimal(start, end) => format!(
                    "{}.pipe(Schema.betweenBigDecimal(BigDecimal.fromNumber({}), BigDecimal.fromNumber({})))",
                    result, start, end
                ),
                BigDecimalValidator::PositiveBigDecimal => {
                    format!("{}.pipe(Schema.positiveBigDecimal())", result)
                }
                BigDecimalValidator::NonNegativeBigDecimal => {
                    format!("{}.pipe(Schema.nonNegativeBigDecimal())", result)
                }
                BigDecimalValidator::NegativeBigDecimal => {
                    format!("{}.pipe(Schema.negativeBigDecimal())", result)
                }
                BigDecimalValidator::NonPositiveBigDecimal => {
                    format!("{}.pipe(Schema.nonPositiveBigDecimal())", result)
                }
            },

            // Duration validators
            Validator::DurationValidator(dv) => match dv {
                DurationValidator::GreaterThanDuration(value) => {
                    format!("{}.pipe(Schema.greaterThanDuration(\"{}\"))", result, value)
                }
                DurationValidator::GreaterThanOrEqualToDuration(value) => format!(
                    "{}.pipe(Schema.greaterThanOrEqualToDuration(\"{}\"))",
                    result, value
                ),
                DurationValidator::LessThanDuration(value) => {
                    format!("{}.pipe(Schema.lessThanDuration(\"{}\"))", result, value)
                }
                DurationValidator::LessThanOrEqualToDuration(value) => format!(
                    "{}.pipe(Schema.lessThanOrEqualToDuration(\"{}\"))",
                    result, value
                ),
                DurationValidator::BetweenDuration(start, end) => format!(
                    "{}.pipe(Schema.betweenDuration(\"{}\", \"{}\"))",
                    result, start, end
                ),
            },
        };
    }

    result
}
