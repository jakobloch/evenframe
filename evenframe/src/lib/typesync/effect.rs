use crate::types::{FieldType, TaggedUnion, VariantData};
use crate::types::StructConfig;
use crate::validator::{
    ArrayValidator, BigDecimalValidator, BigIntValidator, DateValidator, DurationValidator,
    NumberValidator, StringValidator, Validator,
};
use crate::{
 
        dependency::{analyse_recursion, deps_of, RecursionInfo},
       
   
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
    
    // 1.  analyse once
    tracing::debug!("Analyzing recursion in types");
    let rec = analyse_recursion(structs, enums);

    let mut out_encoded = String::new();
    /* every struct gets an interface */
    tracing::debug!("Generating encoded interfaces for structs");
    for sc in structs.values() {
        tracing::trace!(struct_name = %sc.name, "Generating encoded interface");
        out_encoded.push_str(&encoded_interface_for_struct(sc, structs, enums));
    }

    /* every union/enum gets a type alias */
    tracing::debug!("Generating encoded aliases for enums");
    for e in enums.values() {
        tracing::trace!(enum_name = %e.enum_name, "Generating encoded alias");
        out_encoded.push_str(&encoded_alias_for_enum(e, structs, enums));
    }

    // 2.  topologically sort components so all **non-recursive**
    //     dependencies appear first; this removes the need for
    //     `Schema.suspend` outside recursive SCCs.
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
            // helper that returns HashSet<String>
            let to_comp = rec.comp_of[t2];
            if from_comp != to_comp {
                condensation.add_edge(from_comp, to_comp, ());
            }
        }
    }
    // A → B edge means "A depends on B", so topo order is `toposort` reversed
    let mut ordered_comps = toposort(&condensation, None).unwrap_or_default();
    ordered_comps.reverse();

    // 3.  generate ------------------------------------------------------------
    tracing::debug!("Generating schema classes");
    let mut out_classes = String::new();
    let mut out_types = String::new();
    let mut processed = HashSet::<String>::new();

    // helper closure for field conversion that has access to `rec`
    let to_schema = |ft: &FieldType, cur: &str, proc: &HashSet<String>| -> String {
        field_type_to_effect_schema(ft, structs, enums, cur, &rec, &proc)
    };

    for comp_id in ordered_comps {
        // order inside the SCC is arbitrary; preserve original order of input
        // (helps deterministic output)
        let mut members = rec.meta[&comp_id].1.clone();
        members.sort();

        for name in members {
            if let Some(e) = enums
                .values()
                .find(|e| e.enum_name.to_case(Case::Pascal) == name)
            {
                // ---- enum ---------------------------------------------------
                out_classes.push_str(&format!("export const {} = Schema.Union(", name));
                let variants = e
                    .variants
                    .iter()
                    .map(|v| {
                        v.data
                            .as_ref()
                            .map(|variant_data| {
                                match variant_data {
                                    VariantData::InlineStruct(_) => {
                                        // For inline structs, use the variant name directly
                                        v.name.to_case(Case::Pascal)
                                    }
                                    VariantData::DataStructureRef(field_type) => {
                                        to_schema(field_type, &name, &processed)
                                    }
                                }
                            })
                            .unwrap_or_else(|| format!("Schema.Literal(\"{}\")", v.name))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                out_classes.push_str(&variants);
                out_classes.push_str(&format!(").annotations({{ identifier: `{}` }});\n", name));
                out_types.push_str(&format!(
                    "export type {}Type = typeof {}.Type;\n",
                    name, name
                ));
            } else if let Some(sc) = structs.values().find(|sc| sc.name.to_case(Case::Pascal) == name) {
                // ---- struct --------------------------------------------------
                out_classes.push_str(&format!(
                    "export class {} extends Schema.Class<{}>(\"{}\")( {{\n",
                    name, name, name
                ));
                for (idx, f) in sc.fields.iter().enumerate() {
                    let schema = to_schema(&f.field_type, &name, &processed);
                    let schema_with_validators = apply_validators_to_schema(schema, &f.validators);
                    out_classes.push_str(&format!(
                        "  {}: {}{}",
                        f.field_name.to_case(Case::Camel),
                        schema_with_validators,
                        if idx + 1 == sc.fields.len() {
                            '\n'
                        } else {
                            ','
                        }
                    ));
                    if idx + 1 != sc.fields.len() {
                        out_classes.push('\n');
                    }
                }
                out_classes.push_str("}) {[key: string]: unknown}\n\n");
                out_types.push_str(&format!(
                    "export type {}Type = typeof {}.Type;\n",
                    name, name
                ));
            }
            processed.insert(name);
        }
    }

    let result = if print_types {
        format!("{out_classes}\n{out_encoded}\n{out_types}")
    } else {
        format!("{out_classes}\n{out_encoded}")
    };
    
    tracing::info!(output_length = result.len(), "Effect Schema generation complete");
    result
}

// ----- for classes ---------------------------------------------------------
fn encoded_interface_for_struct(
    struct_config: &StructConfig,
    structs: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
) -> String {
    let name = struct_config.name.to_case(Case::Pascal);
    let body = struct_config
        .fields
        .iter()
        .map(|f| {
            format!(
                "  readonly {}: {};",
                f.field_name.to_case(Case::Camel),
                field_type_to_ts_encoded(&f.field_type, structs, enums)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("export interface {name}Encoded {{\n{body}\n}}\n\n")
}

// ----- for unions / enums --------------------------------------------------
fn encoded_alias_for_enum(
    en: &TaggedUnion,
    structs: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
) -> String {
    tracing::trace!(enum_name = %en.enum_name, "Creating encoded alias for enum");
    let name = en.enum_name.to_case(Case::Pascal);
    let union = en
        .variants
        .iter()
        .map(|v| match &v.data {
            Some(variant_data) => {
                match variant_data {
                    VariantData::InlineStruct(_) => {
                        // For inline structs, use the variant name + "Encoded"
                        format!("{}Encoded", v.name.to_case(Case::Pascal))
                    }
                    VariantData::DataStructureRef(field_type) => {
                        field_type_to_ts_encoded(field_type, structs, enums)
                    }
                }
            }
            None => format!("\"{}\"", v.name),
        })
        .collect::<Vec<_>>()
        .join(" | ");

    format!("export type {name}Encoded = {union};\n\n")
}

fn field_type_to_effect_schema(
    field_type: &FieldType,
    structs: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
    current: &str,       // NEW: name of the type we are expanding
    rec: &RecursionInfo, // NEW: recursion helper
    processed: &HashSet<String>,
) -> String {
    // helper to recurse with the same context
    let field = |inner: &FieldType| -> String {
        field_type_to_effect_schema(inner, structs, enums, current, rec, processed)
    };
    match field_type {
        FieldType::String => "Schema.String".to_string(),
        FieldType::Char => "Schema.String".to_string(),
        FieldType::Bool => "Schema.Boolean".to_string(),
        FieldType::Unit => "Schema.Null".to_string(),
        FieldType::Decimal => "Schema.Number".to_string(),
        FieldType::OrderedFloat(_inner) => "Schema.Number".to_string(), // OrderedFloat is treated as number
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
        FieldType::DateTime => "Schema.DateTimeUtc".to_string(),
        FieldType::Duration => "Schema.Number".to_string(), // nanoseconds
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
        FieldType::HashMap(k, v) | FieldType::BTreeMap(k, v) => format!(
            "Schema.Record({{ key: {}, value: {} }})",
            field(k).replace('\'', ""),
            field(v).replace('\'', "")
        ),
        FieldType::Other(name) => {
            let pascal = name.to_case(Case::Pascal);
            let wrap_id = format!("{pascal}Ref");
            // --- decide whether we need Schema.suspend --------------------
            if rec.is_recursive_pair(current, &pascal) && !processed.contains(&pascal) {
                // forward edge *inside* recursive SCC → suspend
                if structs
                    .values()
                    .any(|sc| sc.name.to_case(Case::Pascal) == pascal)
                {
                    format!(
                        "Schema.suspend((): Schema.Schema<{0}, {0}Encoded> => {0}).annotations({{ identifier: `{1}` }})",
                        pascal, wrap_id
                    )
                } else {
                    format!(
                        "Schema.suspend((): Schema.Schema<typeof {0}.Type, {0}Encoded> => {0}).annotations({{ identifier: `{1}` }})",
                        pascal, wrap_id
                    )
                }
            } else {
                // everything else: direct reference
                pascal
            }
        }
    }
}

fn field_type_to_ts_encoded(
    ft: &FieldType,
    structs: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
) -> String {
    // small closure so we don't repeat the arg list everywhere
    let enc = |f: &FieldType| field_type_to_ts_encoded(f, structs, enums);

    match ft {
        // primitives --------------------------------------------------------
        FieldType::String | FieldType::Char | FieldType::EvenframeRecordId => "string".into(),
        FieldType::Bool => "boolean".into(),
        FieldType::DateTime => "string".into(),
        FieldType::Duration => "number".into(), // nanoseconds
        FieldType::Timezone => "string".into(), // IANA timezone string
        FieldType::Unit => "null".into(),
        FieldType::Decimal => "number".to_string(),
        FieldType::OrderedFloat(_inner) => "number".to_string(), // OrderedFloat is treated as number
        FieldType::F32
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

        // containers --------------------------------------------------------
        FieldType::Option(inner) => format!("{} | null | undefined", enc(inner)),
        FieldType::Vec(inner) => format!("ReadonlyArray<{}>", enc(inner)),

        FieldType::Tuple(items) => {
            let elems = items.iter().map(enc).collect::<Vec<_>>().join(", ");
            format!("[{}]", elems)
        }

        FieldType::Struct(fs) => {
            let body = fs
                .iter()
                .map(|(n, f)| format!("readonly {}: {}", n, enc(f)))
                .collect::<Vec<_>>()
                .join("; ");
            format!("{{ {} }}", body)
        }

        FieldType::HashMap(k, v) | FieldType::BTreeMap(k, v) => {
            format!("Record<{}, {}>", enc(k), enc(v))
        }

        FieldType::RecordLink(inner) => format!("string | {}", enc(inner)),

        // other user-defined -------------------------------------------------
        FieldType::Other(name) => format!("{}Encoded", name.to_case(Case::Pascal)),
    }
}

fn apply_validators_to_schema(schema: String, validators: &[Validator]) -> String {
    if validators.is_empty() {
        return schema;
    }

    let mut result = schema;
    
    for validator in validators {
        result = match validator {
            // String validators
            Validator::StringValidator(sv) => match sv {
                StringValidator::MinLength(len) => format!("{}.pipe(Schema.minLength({}))", result, len),
                StringValidator::MaxLength(len) => format!("{}.pipe(Schema.maxLength({}))", result, len),
                StringValidator::Length(len) => format!("{}.pipe(Schema.length({}))", result, len),
                StringValidator::NonEmpty => format!("{}.pipe(Schema.nonEmptyString())", result),
                StringValidator::StartsWith(prefix) => format!("{}.pipe(Schema.startsWith(\"{}\"))", result, prefix),
                StringValidator::EndsWith(suffix) => format!("{}.pipe(Schema.endsWith(\"{}\"))", result, suffix),
                StringValidator::Includes(substring) => format!("{}.pipe(Schema.includes(\"{}\"))", result, substring),
                StringValidator::Trimmed => format!("{}.pipe(Schema.trimmed())", result),
                StringValidator::Lowercased => format!("{}.pipe(Schema.lowercased())", result),
                StringValidator::Uppercased => format!("{}.pipe(Schema.uppercased())", result),
                StringValidator::Capitalized => format!("{}.pipe(Schema.capitalized())", result),
                StringValidator::Uncapitalized => format!("{}.pipe(Schema.uncapitalized())", result),
                StringValidator::RegexLiteral(format_variant) => format!("{}.pipe(Schema.pattern(/{}/))", result, format_variant.to_owned().into_regex().as_str()),
                _ => result, // Other validators don't have direct Effect Schema equivalents
            },
            
            // Number validators
            Validator::NumberValidator(nv) => match nv {
                NumberValidator::GreaterThan(value) => format!("{}.pipe(Schema.greaterThan({}))", result, value.0),
                NumberValidator::GreaterThanOrEqualTo(value) => format!("{}.pipe(Schema.greaterThanOrEqualTo({}))", result, value.0),
                NumberValidator::LessThan(value) => format!("{}.pipe(Schema.lessThan({}))", result, value.0),
                NumberValidator::LessThanOrEqualTo(value) => format!("{}.pipe(Schema.lessThanOrEqualTo({}))", result, value.0),
                NumberValidator::Between(start, end) => format!("{}.pipe(Schema.between({}, {}))", result, start.0, end.0),
                NumberValidator::Int => format!("{}.pipe(Schema.int())", result),
                NumberValidator::NonNaN => format!("{}.pipe(Schema.nonNaN())", result),
                NumberValidator::Finite => format!("{}.pipe(Schema.finite())", result),
                NumberValidator::Positive => format!("{}.pipe(Schema.positive())", result),
                NumberValidator::NonNegative => format!("{}.pipe(Schema.nonNegative())", result),
                NumberValidator::Negative => format!("{}.pipe(Schema.negative())", result),
                NumberValidator::NonPositive => format!("{}.pipe(Schema.nonPositive())", result),
                NumberValidator::MultipleOf(value) => format!("{}.pipe(Schema.multipleOf({}))", result, value.0),
                NumberValidator::Uint8 => result, // Schema.Uint8 should be used as base type instead
            },
            
            // Array validators
            Validator::ArrayValidator(av) => match av {
                ArrayValidator::MinItems(count) => format!("{}.pipe(Schema.minItems({}))", result, count),
                ArrayValidator::MaxItems(count) => format!("{}.pipe(Schema.maxItems({}))", result, count),
                ArrayValidator::ItemsCount(count) => format!("{}.pipe(Schema.itemsCount({}))", result, count),
            },
            
            // Date validators
            Validator::DateValidator(dv) => match dv {
                DateValidator::ValidDate => format!("{}.pipe(Schema.validDate())", result),
                DateValidator::GreaterThanDate(date) => format!("{}.pipe(Schema.greaterThanDate(new Date(\"{}\")))", result, date),
                DateValidator::GreaterThanOrEqualToDate(date) => format!("{}.pipe(Schema.greaterThanOrEqualToDate(new Date(\"{}\")))", result, date),
                DateValidator::LessThanDate(date) => format!("{}.pipe(Schema.lessThanDate(new Date(\"{}\")))", result, date),
                DateValidator::LessThanOrEqualToDate(date) => format!("{}.pipe(Schema.lessThanOrEqualToDate(new Date(\"{}\")))", result, date),
                DateValidator::BetweenDate(start, end) => format!("{}.pipe(Schema.betweenDate(new Date(\"{}\"), new Date(\"{}\")))", result, start, end),
            },
            
            // BigInt validators
            Validator::BigIntValidator(biv) => match biv {
                BigIntValidator::GreaterThanBigInt(value) => format!("{}.pipe(Schema.greaterThanBigInt({}n))", result, value),
                BigIntValidator::GreaterThanOrEqualToBigInt(value) => format!("{}.pipe(Schema.greaterThanOrEqualToBigInt({}n))", result, value),
                BigIntValidator::LessThanBigInt(value) => format!("{}.pipe(Schema.lessThanBigInt({}n))", result, value),
                BigIntValidator::LessThanOrEqualToBigInt(value) => format!("{}.pipe(Schema.lessThanOrEqualToBigInt({}n))", result, value),
                BigIntValidator::BetweenBigInt(start, end) => format!("{}.pipe(Schema.betweenBigInt({}n, {}n))", result, start, end),
                BigIntValidator::PositiveBigInt => format!("{}.pipe(Schema.positiveBigInt())", result),
                BigIntValidator::NonNegativeBigInt => format!("{}.pipe(Schema.nonNegativeBigInt())", result),
                BigIntValidator::NegativeBigInt => format!("{}.pipe(Schema.negativeBigInt())", result),
                BigIntValidator::NonPositiveBigInt => format!("{}.pipe(Schema.nonPositiveBigInt())", result),
            },
            
            // BigDecimal validators
            Validator::BigDecimalValidator(bdv) => match bdv {
                BigDecimalValidator::GreaterThanBigDecimal(value) => format!("{}.pipe(Schema.greaterThanBigDecimal(BigDecimal.unsafeFromNumber({})))", result, value),
                BigDecimalValidator::GreaterThanOrEqualToBigDecimal(value) => format!("{}.pipe(Schema.greaterThanOrEqualToBigDecimal(BigDecimal.unsafeFromNumber({})))", result, value),
                BigDecimalValidator::LessThanBigDecimal(value) => format!("{}.pipe(Schema.lessThanBigDecimal(BigDecimal.unsafeFromNumber({})))", result, value),
                BigDecimalValidator::LessThanOrEqualToBigDecimal(value) => format!("{}.pipe(Schema.lessThanOrEqualToBigDecimal(BigDecimal.unsafeFromNumber({})))", result, value),
                BigDecimalValidator::BetweenBigDecimal(start, end) => format!("{}.pipe(Schema.betweenBigDecimal(BigDecimal.unsafeFromNumber({}), BigDecimal.unsafeFromNumber({})))", result, start, end),
                BigDecimalValidator::PositiveBigDecimal => format!("{}.pipe(Schema.positiveBigDecimal())", result),
                BigDecimalValidator::NonNegativeBigDecimal => format!("{}.pipe(Schema.nonNegativeBigDecimal())", result),
                BigDecimalValidator::NegativeBigDecimal => format!("{}.pipe(Schema.negativeBigDecimal())", result),
                BigDecimalValidator::NonPositiveBigDecimal => format!("{}.pipe(Schema.nonPositiveBigDecimal())", result),
            },
            
            // Duration validators
            Validator::DurationValidator(dv) => match dv {
                DurationValidator::GreaterThanDuration(value) => format!("{}.pipe(Schema.greaterThanDuration(\"{}\"))", result, value),
                DurationValidator::GreaterThanOrEqualToDuration(value) => format!("{}.pipe(Schema.greaterThanOrEqualToDuration(\"{}\"))", result, value),
                DurationValidator::LessThanDuration(value) => format!("{}.pipe(Schema.lessThanDuration(\"{}\"))", result, value),
                DurationValidator::LessThanOrEqualToDuration(value) => format!("{}.pipe(Schema.lessThanOrEqualToDuration(\"{}\"))", result, value),
                DurationValidator::BetweenDuration(start, end) => format!("{}.pipe(Schema.betweenDuration(\"{}\", \"{}\"))", result, start, end),
            },
        }
    }
    
    result
}
