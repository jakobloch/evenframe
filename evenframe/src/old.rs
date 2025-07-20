use chrono::{DateTime, Duration, SecondsFormat, Timelike, Utc};
use dotenv::dotenv;
use handlers::{
    account::{Account, Ordered},
    appointment::{
        Appointment, AppointmentFieldEnum, Colors, ColorsFieldEnum, DailyRecurrenceRule, Interval,
        MonthlyRecurrenceRule, RecurrenceEnd, RecurrenceRule, Status, WeekOfMonth, Weekday,
        WeeklyRecurrenceRule, YearlyRecurrenceRule,
    },
    company::Company,
    db::Table,
    employee::{Employee, Represents},
    lead::{Lead, LeadStage, NextStep, Priority},
    order::{BilledItem, Item, Order, OrderStage, Package, Payment, Promotion},
    product::{Product, ProductDefaults},
    route::Route,
    service::{Service, ServiceDefaults},
    tax_rate::TaxRate,
    user::{
        AppPermissions, Applications, AppointmentNotifications, Commissions, Metadata, Page,
        Settings, User, UserRole,
    },
    validation::account_validation::{
        Color, Coordinates, PhoneNumber, PhoneNumberFieldEnum, Sector, Site,
    },
    Email,
};
use helpers::{
    case::{to_camel_case, to_pascal_case},
    schemasync::*,
};
use rand::{rng, seq::IndexedRandom, Rng};
use rand_distr::Alphanumeric;
use regex::Regex;
// use regex::Regex;
use serde_json::{to_string, Value};
use specta::ts::{self, ExportConfiguration};

use derive_more::From;
use petgraph::algo::{kosaraju_scc, toposort};
use petgraph::graphmap::DiGraphMap;
use std::{
    collections::{HashMap, HashSet},
    env,
};
use surrealdb::{engine::remote::ws::Ws, opt::auth::Root, Surreal};

/// ------------------------------------------------------------------
///  A tiny helper you can compute once and pass around afterwards
/// ------------------------------------------------------------------
#[derive(Debug)]
pub struct RecursionInfo {
    /// `type_name → scc_id`
    comp_of: HashMap<String, usize>,
    /// `scc_id → { "is_recursive": bool, "members": Vec<String> }`
    meta: HashMap<usize, (bool, Vec<String>)>,
}

impl RecursionInfo {
    /// true when current & target are in the **same** SCC and that SCC
    ///        is either larger than 1   **or**   has a self-loop
    fn is_recursive_pair(&self, current: &str, target: &str) -> bool {
        let c_id = self.comp_of.get(current);
        let t_id = self.comp_of.get(target);
        match (c_id, t_id) {
            (Some(c), Some(t)) if c == t => self.meta[c].0, // same comp & recursive
            _ => false,
        }
    }
}

/// ------------------------------------------------------------------
///  Build the dependency graph from your `FieldType` tree
/// ------------------------------------------------------------------
pub fn analyse_recursion(tables: &[Schema], enums: &[TaggedUnion]) -> RecursionInfo {
    // --- collect dependencies ------------------------------------------------
    fn collect_refs(ft: &FieldType, known: &HashSet<String>, acc: &mut HashSet<String>) {
        use FieldType::*;
        match ft {
            Tuple(v) => v.iter().for_each(|f| collect_refs(f, known, acc)),
            Struct(v) => v.iter().for_each(|(_, f)| collect_refs(f, known, acc)),
            Option(i) | Vec(i) | RecordLink(i) => collect_refs(i, known, acc),
            HashMap(k, v) | BTreeMap(k, v) => {
                collect_refs(k, known, acc);
                collect_refs(v, known, acc);
            }
            Other(name) if known.contains(name) => {
                acc.insert(name.clone());
            }
            _ => {}
        }
    }

    let known: HashSet<_> = tables
        .iter()
        .map(|t| to_pascal_case(&t.table_schema.table_name))
        .chain(enums.iter().map(|e| to_pascal_case(&e.enum_name)))
        .collect();

    let mut deps: HashMap<String, HashSet<String>> = HashMap::new();

    for t in tables {
        let from = to_pascal_case(&t.table_schema.table_name);
        let entry = deps.entry(from.clone()).or_default();
        for f in &t.table_schema.fields {
            collect_refs(&f.field_type, &known, entry);
        }
    }
    for e in enums {
        let from = to_pascal_case(&e.enum_name);
        let entry = deps.entry(from.clone()).or_default();
        for v in &e.variants {
            if let Some(ft) = &v.data {
                collect_refs(ft, &known, entry);
            }
        }
    }

    // --- build graph ---------------------------------------------------------
    let mut g: DiGraphMap<&str, ()> = DiGraphMap::new();
    for (from, tos) in &deps {
        // ensure node exists even if it has no outgoing edges
        g.add_node(from.as_str());
        for to in tos {
            g.add_edge(from.as_str(), to.as_str(), ());
        }
    }

    // --- strongly connected components --------------------------------------
    let sccs = kosaraju_scc(&g); // Vec<Vec<&str>>

    let mut comp_of = HashMap::<String, usize>::new();
    let mut meta = HashMap::<usize, (bool, Vec<String>)>::new();

    for (idx, comp) in sccs.iter().enumerate() {
        let self_loop = comp.len() == 1 && g.contains_edge(comp[0], comp[0]);
        let recursive = self_loop || comp.len() > 1;
        let members = comp.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        for m in &members {
            comp_of.insert(m.clone(), idx);
        }
        meta.insert(idx, (recursive, members));
    }

    RecursionInfo { comp_of, meta }
}

#[derive(Debug, Clone, PartialEq, Eq, From)]
enum Requirements {
    StringRequirements(StringRequirements),
    NumberRequirements(NumberRequirements),
}

/// Describes various string validation and transformation requirements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringRequirements {
    /// A string
    String,

    /// Only letters
    Alpha,

    /// Only letters and digits 0-9
    Alphanumeric,

    /// Base64-encoded
    Base64,

    /// Base64url-encoded
    Base64Url,

    /// A morph from a string to capitalized
    Capitalize,

    /// Capitalized
    CapitalizePreformatted,

    /// A credit card number and a credit card number
    CreditCard,

    /// A string and a parsable date
    Date,

    /// An integer string representing a safe Unix timestamp
    DateEpoch,

    /// A morph from an integer string representing a safe Unix timestamp to a Date
    DateEpochParse,

    /// An ISO 8601 (YYYY-MM-DDTHH:mm:ss.sssZ) date
    DateIso,

    /// A morph from an ISO 8601 (YYYY-MM-DDTHH:mm:ss.sssZ) date to a Date
    DateIsoParse,

    /// A morph from a string and a parsable date to a Date
    DateParse,

    /// Only digits 0-9
    Digits,

    /// An email address
    Email,

    /// Hex characters only
    Hex,

    /// A well-formed integer string
    Integer,

    /// A morph from a well-formed integer string to an integer
    IntegerParse,

    /// An IP address
    Ip,

    /// An IPv4 address
    IpV4,

    /// An IPv6 address
    IpV6,

    /// A JSON string
    Json,

    /// Safe JSON string parser
    JsonParse,

    /// A morph from a string to only lowercase letters
    Lower,

    /// Only lowercase letters
    LowerPreformatted,

    /// A morph from a string to NFC-normalized unicode
    Normalize,

    /// A morph from a string to NFC-normalized unicode
    NormalizeNFC,

    /// NFC-normalized unicode
    NormalizeNFCPreformatted,

    /// A morph from a string to NFD-normalized unicode
    NormalizeNFD,

    /// NFD-normalized unicode
    NormalizeNFDPreformatted,

    /// A morph from a string to NFKC-normalized unicode
    NormalizeNFKC,

    /// NFKC-normalized unicode
    NormalizeNFKCPreformatted,

    /// A morph from a string to NFKD-normalized unicode
    NormalizeNFKD,

    /// NFKD-normalized unicode
    NormalizeNFKDPreformatted,

    /// A well-formed numeric string
    Numeric,

    /// A morph from a well-formed numeric string to a number
    NumericParse,

    /// A string and a regex pattern
    Regex,

    /// A semantic version (see https://semver.org/)
    Semver,

    /// A morph from a string to trimmed
    Trim,

    /// Trimmed
    TrimPreformatted,

    /// A morph from a string to only uppercase letters
    Upper,

    /// Only uppercase letters
    UpperPreformatted,

    /// A string and a URL string
    Url,

    /// A morph from a string and a URL string to a URL instance
    UrlParse,

    /// A UUID
    Uuid,

    /// A UUIDv1
    UuidV1,

    /// A UUIDv2
    UuidV2,

    /// A UUIDv3
    UuidV3,

    /// A UUIDv4
    UuidV4,

    /// A UUIDv5
    UuidV5,

    /// A UUIDv6
    UuidV6,

    /// A UUIDv7
    UuidV7,

    /// A UUIDv8
    UuidV8,

    Literal(String),

    StringEmbedded(String),

    RegexLiteral(String),

    Length(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberRequirements {}
pub struct Schema {
    table_schema: TableSchema,
    requirements: Option<HashMap<FieldEnum, Vec<Requirements>>>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    specta::Type,
    serde::Serialize,
    serde::Deserialize,
    Hash,
    From,
)]
pub enum FieldEnum {
    AppointmentFieldEnum(AppointmentFieldEnum),
    PhoneNumberFieldEnum(PhoneNumberFieldEnum),
    ColorsFieldEnum(ColorsFieldEnum),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generate_dummy_values = false;
    let generate_arktype_types = false;
    let generate_effect_schemas = true;
    let generate_specta_types = false;

    if generate_specta_types {
        let ts_types = collect_specta_types().join("\n");

        std::fs::write(
            "../../frontend/src/lib/core/types/specta.ts",
            format!("import {{ RecordId }} from 'surrealdb';\n\n{}", &ts_types,),
        )?;
    }

    if generate_arktype_types {
        std::fs::write(
            "../../frontend/src/lib/core/types/arktype.ts",
            format!(
                "import {{ scope }} from 'arktype';\n\n{}\n\n export const validator = scope({{
  ...bindings.export(),
            }}).export();",
                generate_arktype_type_string(
                    &[
                        Schema {
                            table_schema: PhoneNumber::table_schema(),
                            requirements: Some(HashMap::from([
                                (PhoneNumberFieldEnum::Number.into(), vec![
                                StringRequirements::RegexLiteral(
                                    "^(\\+\\d{1,2}\\s?)?\\(?\\d{3}\\)?[\\s.\\-]?\\d{3}[\\s.\\-]?\\d{4}$"
                                        .to_owned(),
                                ).into(),
                            ])
                            ])
                            ),
                        },
                        Schema {
                            table_schema: Coordinates::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Commissions::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Settings::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Colors::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Email::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: BilledItem::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: AppointmentNotifications::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Metadata::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Color::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: ServiceDefaults::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: ProductDefaults::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Represents::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Ordered::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Account::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Appointment::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Lead::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: TaxRate::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Site::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Employee::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Route::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Company::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Product::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Service::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: User::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Order::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Payment::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Package::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Promotion::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: RecurrenceRule::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: MonthlyRecurrenceRule::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: WeeklyRecurrenceRule::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: DailyRecurrenceRule::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: YearlyRecurrenceRule::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: AppPermissions::table_schema(),
                            requirements: None,
                        },

                    ]
                    ,
                    &[
                        Sector::variants(),
                        LeadStage::variants(),
                        NextStep::variants(),
                        Priority::variants(),
                        OrderStage::variants(),
                        Item::variants(),
                        Status::variants(),
                        WeekOfMonth::variants(),
                        Weekday::variants(),
                        RecurrenceEnd::variants(),
                        Interval::variants(),
                        Table::variants(),
                        Page::variants(),
                        Applications::variants(),
                        UserRole::variants(),
                    ],
                    false
                ),
            ),
        )?;
    }

    if generate_effect_schemas {
        std::fs::write(
            "../../frontend/src/lib/core/types/bindings.ts",
            format!(
                "import {{ Schema }} from \"effect\";\n\n{}",
                generate_effect_schema_string(
                    &[
                        Schema {
                            table_schema: PhoneNumber::table_schema(),
                            requirements: Some(HashMap::from([
                                (PhoneNumberFieldEnum::Number.into(), vec![
                                StringRequirements::RegexLiteral(
                                    "^(\\+\\d{1,2}\\s?)?\\(?\\d{3}\\)?[\\s.\\-]?\\d{3}[\\s.\\-]?\\d{4}$"
                                        .to_owned(),
                                ).into(),
                            ])
                            ])
                            ),
                        },
                        Schema {
                            table_schema: Coordinates::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Commissions::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Settings::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Colors::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Email::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: BilledItem::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: AppointmentNotifications::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Metadata::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Color::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: ServiceDefaults::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: ProductDefaults::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Represents::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Ordered::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Account::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Appointment::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Lead::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: TaxRate::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Site::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Employee::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Route::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Company::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Product::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Service::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: User::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Order::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Payment::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Package::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: Promotion::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: RecurrenceRule::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: MonthlyRecurrenceRule::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: WeeklyRecurrenceRule::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: DailyRecurrenceRule::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: YearlyRecurrenceRule::table_schema(),
                            requirements: None,
                        },
                        Schema {
                            table_schema: AppPermissions::table_schema(),
                            requirements: None,
                        },

                    ]
                    ,
                    &[
                        Sector::variants(),
                        LeadStage::variants(),
                        NextStep::variants(),
                        Priority::variants(),
                        OrderStage::variants(),
                        Item::variants(),
                        Status::variants(),
                        WeekOfMonth::variants(),
                        Weekday::variants(),
                        RecurrenceEnd::variants(),
                        Interval::variants(),
                        Table::variants(),
                        Page::variants(),
                        Applications::variants(),
                        UserRole::variants(),
                    ],
                    false
                ),
            ),
        )?;
    }

    if generate_dummy_values {
        dotenv().ok(); // Load .env file

        match Surreal::new::<Ws>(env::var("SURREAL_URL").expect("Surreal URL not set")).await {
            Ok(db) => {
                db.signin(Root {
                    username: "root",
                    password: "root",
                })
                .await?;

                db.use_ns("avel").use_db("test").await?;
                generate_all_inserts(
                    db,
                    &[
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Account') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Account') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Account') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Account') = true",
                            ),
                            schema: Account::table_schema(),
                            n: 13,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Appointment') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Appointment') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Appointment') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Appointment') = true",
                            ),
                            schema: Appointment::table_schema(),
                            n: 13,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Lead') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Lead') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Lead') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Lead') = true",
                            ),
                            schema: Lead::table_schema(),
                            n: 4,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('TaxRate') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('TaxRate') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('TaxRate') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('TaxRate') = true",
                            ),
                            schema: TaxRate::table_schema(),
                            n: 3,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Site') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Site') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Site') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Site') = true",
                            ),
                            schema: Site::table_schema(),
                            n: 50,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Employee') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Employee') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Employee') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Employee') = true",
                            ),
                            schema: Employee::table_schema(),
                            n: 32,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Route') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Route') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Route') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Route') = true",
                            ),
                            schema: Route::table_schema(),
                            n: 10,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Company') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Company') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Company') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Company') = true",
                            ),
                            schema: Company::table_schema(),
                            n: 1,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Product') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Product') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Product') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Product') = true",
                            ),
                            schema: Product::table_schema(),
                            n: 4,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Service') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Service') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Service') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Service') = true",
                            ),
                            schema: Service::table_schema(),
                            n: 12,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('User') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('User') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('User') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('User') = true",
                            ),
                            schema: User::table_schema(),
                            n: 15,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Order') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Order') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Order') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Order') = true",
                            ),
                            schema: Order::table_schema(),
                            n: 20,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Payment') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Payment') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Payment') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Payment') = true",
                            ),
                            schema: Payment::table_schema(),
                            n: 4,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Package') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Package') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Package') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Package') = true",
                            ),
                            schema: Package::table_schema(),
                            n: 2,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Promotion') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Promotion') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Promotion') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Promotion') = true",
                            ),
                            schema: Promotion::table_schema(),
                            n: 4,
                            relation: None,
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Represents') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Represents') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Represents') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Represents') = true",
                            ),
                            schema: Represents::table_schema(),
                            n: 13,
                            relation: Some(EdgeConfig {
                                edge_name: "represents".to_string(),
                                from: "employee".to_string(),
                                to: "account".to_string(),
                                direction: Direction::To,
                            }),
                        },
                        QueryGenerationConfig {
                            select_permissions: Some(
                                "WHERE $auth.permissions.data.any('Ordered') = true",
                            ),
                            update_permissions: Some(
                                "WHERE $auth.permissions.data.any('Ordered') = true",
                            ),
                            create_permissions: Some(
                                "WHERE $auth.permissions.data.any('Ordered') = true",
                            ),
                            delete_permissions: Some(
                                "WHERE $auth.permissions.data.any('Ordered') = true",
                            ),
                            schema: Ordered::table_schema(),
                            n: 13,
                            relation: Some(EdgeConfig {
                                edge_name: "ordered".to_string(),
                                from: "account".to_string(),
                                to: "order".to_string(),
                                direction: Direction::To,
                            }),
                        },
                    ],
                    &[
                        PhoneNumber::table_schema(),
                        Coordinates::table_schema(),
                        Settings::table_schema(),
                        Colors::table_schema(),
                        Email::table_schema(),
                        BilledItem::table_schema(),
                        AppointmentNotifications::table_schema(),
                        Commissions::table_schema(),
                        Metadata::table_schema(),
                        Settings::table_schema(),
                        Color::table_schema(),
                        ServiceDefaults::table_schema(),
                        ProductDefaults::table_schema(),
                        RecurrenceRule::table_schema(),
                        MonthlyRecurrenceRule::table_schema(),
                        WeeklyRecurrenceRule::table_schema(),
                        DailyRecurrenceRule::table_schema(),
                        YearlyRecurrenceRule::table_schema(),
                        AppPermissions::table_schema(),
                    ],
                    &[
                        Sector::variants(),
                        LeadStage::variants(),
                        NextStep::variants(),
                        Priority::variants(),
                        OrderStage::variants(),
                        Item::variants(),
                        Status::variants(),
                        WeekOfMonth::variants(),
                        Weekday::variants(),
                        RecurrenceEnd::variants(),
                        Interval::variants(),
                        Table::variants(),
                        Page::variants(),
                        Applications::variants(),
                        UserRole::variants(),
                    ],
                    HashMap::from([
                        (
                            "email_string".to_string(),
                            vec![
                                "jane.doe@example.com".to_string(),
                                "marco_polo@explorer.net".to_string(),
                                "pixel.artist@digitalrealm.io".to_string(),
                                "nova_spectra@stellarmail.xyz".to_string(),
                                "rusty.code@devtest.org".to_string(),
                                "luna_nightshade@moonbase.co".to_string(),
                                "quantumleap88@futuremail.tech".to_string(),
                                "aurora.borealis@northernlights.email".to_string(),
                                "cipher.secure@encrypted.pro".to_string(),
                                "oliver.twist@literarymail.fake".to_string(),
                                "data.stream@infinity.cloud".to_string(),
                                "violet.sky@rainbowbridge.art".to_string(),
                                "testuser_2023@debugmail.test".to_string(),
                                "neptune.blue@oceandepth.sim".to_string(),
                                "echo.chamber@sonicwave.audio".to_string(),
                                "widget_maker@factorytools.demo".to_string(),
                                "shadow.fox@stealthmode.cloak".to_string(),
                                "byte_size@memorylane.ram".to_string(),
                                "temp.user@placeholder.tmp".to_string(),
                                "fractal.pattern@recursive.design".to_string(),
                                "midnight.owl@nightshift.work".to_string(),
                                "solar.flare@cosmicburst.energy".to_string(),
                                "placeholder_01@dummyaddress.fake".to_string(),
                                "quantum.quark@particlelab.science".to_string(),
                                "static.noise@analogradio.fm".to_string(),
                                "alpha.tester@betaphase.debug".to_string(),
                                "binary.choice@boolean.logic".to_string(),
                                "phantom.operator@ghostprotocol.secure".to_string(),
                                "frost.petal@wintergarden.bot".to_string(),
                                "sample.user@mockdata.dev".to_string(),
                                "void.walker@nullspace.void".to_string(),
                                "pixel.pusher@8bitarcade.retro".to_string(),
                                "echo.alpha@resonance.freq".to_string(),
                                "mockturtle@wonderland.fiction".to_string(),
                                "prism.light@spectralab.optics".to_string(),
                                "placeholder_agent@coveridentity.spy".to_string(),
                                "nebula.dust@stardust.collector".to_string(),
                                "test_case_42@qa.verification".to_string(),
                                "cipher.block@encryption.vault".to_string(),
                                "temp_profile@ephemeral.expire".to_string(),
                                "flux.capacitor@timemachine.tech".to_string(),
                                "virtual.avatar@simulation.world".to_string(),
                                "placeholder_bot@automated.dummy".to_string(),
                                "echo.chamber@reverb.sound".to_string(),
                                "quantum.entanglement@spooky.action".to_string(),
                                "synthetic.human@artificial.life".to_string(),
                                "placeholder_npc@gametest.character".to_string(),
                                "ghost.in.shell@cyberspace.matrix".to_string(),
                                "test.dummy@crashreport.debug".to_string(),
                                "void@null.nil".to_string(),
                            ],
                        ),
                        (
                            "begins".to_string(),
                            generate_vec(50, || generate_random_date(7)),
                        ),
                        ("duration".to_string(), generate_iso8601_durations(50)),
                        (
                            "timezone".to_string(),
                            vec!["America/Los_Angeles".to_string()],
                        ),
                        (
                            "colors".to_string(),
                            generate_vec(50, || to_string(&generate_hex_colors(None)).unwrap()),
                        ),
                        (
                            "recurrence_rule".to_string(),
                            generate_vec(50, || generate_random_recurrence_rule_json()),
                        ), // (
                           //     "offset_ms".to_string(),
                           //     get_offset_ms("America/Los_Angeles", Utc::now()),
                           // ),
                    ]),
                )
                .await?;
            }
            Err(e) => {
                eprintln!("Database connection error: {}", e);
                return Err(e.into());
            }
        }
    }

    Ok(())
}
/// Returns the set of **direct** dependencies of a type name
/// (other tables / enums that it references in its fields or variants).
fn deps_of(name: &str, tables: &[Schema], enums: &[TaggedUnion]) -> HashSet<String> {
    // helper reused from `analyse_recursion`
    fn collect_refs(ft: &FieldType, known: &HashSet<String>, acc: &mut HashSet<String>) {
        use FieldType::*;
        match ft {
            Tuple(v) => v.iter().for_each(|f| collect_refs(f, known, acc)),
            Struct(v) => v.iter().for_each(|(_, f)| collect_refs(f, known, acc)),
            Option(i) | Vec(i) | RecordLink(i) => collect_refs(i, known, acc),
            HashMap(k, v) | BTreeMap(k, v) => {
                collect_refs(k, known, acc);
                collect_refs(v, known, acc);
            }
            Other(name) if known.contains(name) => {
                acc.insert(name.clone());
            }
            _ => {} // primitives
        }
    }

    // ----------------------------------------------------------------
    // Build a quick “known-types” set so we don’t count primitives.
    // ----------------------------------------------------------------
    let known: HashSet<_> = tables
        .iter()
        .map(|t| to_pascal_case(&t.table_schema.table_name))
        .chain(enums.iter().map(|e| to_pascal_case(&e.enum_name)))
        .collect();

    let mut acc = HashSet::new();

    // ----------------------------------------------------------------
    // If `name` is a table, walk its fields
    // ----------------------------------------------------------------
    if let Some(t) = tables
        .iter()
        .find(|t| to_pascal_case(&t.table_schema.table_name) == name)
    {
        for f in &t.table_schema.fields {
            collect_refs(&f.field_type, &known, &mut acc);
        }
    }

    // ----------------------------------------------------------------
    // If `name` is an enum, walk its variants
    // ----------------------------------------------------------------
    if let Some(e) = enums.iter().find(|e| to_pascal_case(&e.enum_name) == name) {
        for v in &e.variants {
            if let Some(ft) = &v.data {
                collect_refs(ft, &known, &mut acc);
            }
        }
    }

    acc
}
pub fn generate_effect_schema_string(
    tables: &[Schema],
    enums: &[TaggedUnion],
    print_types: bool,
) -> String {
    // 1.  analyse once
    let rec = analyse_recursion(tables, enums);

    // 2.  topologically sort components so all **non-recursive**
    //     dependencies appear first; this removes the need for
    //     `Schema.suspend` outside recursive SCCs.
    let mut condensation = DiGraphMap::<usize, ()>::new();
    for (t1, _tos) in rec
        .meta
        .values()
        .flat_map(|(_, mem)| mem.iter())
        .filter_map(|n| rec.comp_of.get(n).map(|&c| (n, c)))
    {
        let from_comp = rec.comp_of[t1];
        for t2 in &deps_of(t1, tables, enums) {
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
    let mut out_classes = String::new();
    let mut out_types = String::new();
    let mut processed = HashSet::<String>::new();

    // helper closure for field conversion that has access to `rec`
    let to_schema = |ft: &FieldType, cur: &str, proc: &HashSet<String>| -> String {
        field_type_to_effect_schema(ft, tables, enums, cur, &rec, &proc)
    };

    for comp_id in ordered_comps {
        // order inside the SCC is arbitrary; preserve original order of input
        // (helps deterministic output)
        let mut members = rec.meta[&comp_id].1.clone();
        members.sort();

        for name in members {
            if let Some(e) = enums.iter().find(|e| to_pascal_case(&e.enum_name) == name) {
                // ---- enum ---------------------------------------------------
                out_classes.push_str(&format!("export const {} = Schema.Union(", name));
                let variants = e
                    .variants
                    .iter()
                    .map(|v| {
                        v.data
                            .as_ref()
                            .map(|d| to_schema(d, &name, &processed))
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
            } else if let Some(t) = tables
                .iter()
                .find(|t| to_pascal_case(&t.table_schema.table_name) == name)
            {
                // ---- table --------------------------------------------------
                out_classes.push_str(&format!(
                    "export class {} extends Schema.Class<{}>(\"{}\")( {{\n",
                    name, name, name
                ));
                for (idx, f) in t.table_schema.fields.iter().enumerate() {
                    out_classes.push_str(&format!(
                        "  {}: {}{}",
                        to_camel_case(&f.field_name),
                        to_schema(&f.field_type, &name, &processed),
                        if idx + 1 == t.table_schema.fields.len() {
                            '\n'
                        } else {
                            ','
                        }
                    ));
                    if idx + 1 != t.table_schema.fields.len() {
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

    if print_types {
        format!("{}\n{}", out_classes, out_types)
    } else {
        out_classes
    }
}

fn generate_arktype_type_string(
    tables: &[Schema],
    enums: &[TaggedUnion],
    print_types: bool,
) -> String {
    let mut output = String::new();
    let mut scope_output = String::new();
    let mut types_output = String::new();
    let mut defaults_output = String::new();

    scope_output.push_str("export const bindings = scope({\n\n");

    // First, process all enums
    for schema_enum in enums {
        // Write the Arktype binding name
        scope_output.push_str(&format!("{}: ", to_pascal_case(&schema_enum.enum_name)));

        // We'll accumulate the "nesting" into this string.
        let mut union_ast = String::new();

        for (i, variant) in schema_enum.variants.iter().enumerate() {
            // Convert the variant into either a data type or a literal union piece
            let item_str = if let Some(data) = &variant.data {
                field_type_to_arktype(data, tables, enums)
            } else {
                // For simple string variants, e.g. ["===", "Residential"]
                format!("['===', '{}']", variant.name)
            };

            // If this is our first variant, it becomes the entire union so far,
            // otherwise we nest the "union so far" together with the new item.
            if i == 0 {
                union_ast = item_str;
            } else {
                union_ast = format!("[{}, '|', {}]", union_ast, item_str);
            }
        }

        // Now write out the final folded union string in your scope output
        scope_output.push_str(&format!("{},\n", union_ast));

        // And write the corresponding TypeScript type
        types_output.push_str(&format!(
            "export type {} = typeof bindings.{}.infer;\n",
            to_pascal_case(&schema_enum.enum_name),
            to_pascal_case(&schema_enum.enum_name)
        ));
    }

    // Then, process all tables
    for table in tables {
        let type_name = to_pascal_case(&table.table_schema.table_name);
        scope_output.push_str(&format!("{}: {{\n", type_name));
        defaults_output.push_str(&format!(
            "export const default{}: {} = {{\n",
            &type_name, &type_name
        ));

        for field in &table.table_schema.fields {
            let field_name = to_camel_case(&field.field_name);

            scope_output.push_str(&format!(
                "  {}: {}",
                field_name,
                field_type_to_arktype(&field.field_type, tables, enums)
            ));
            defaults_output.push_str(&format!(
                "{}: {}",
                field_name,
                field_type_to_default_value(&field.field_type, tables, enums)
            ));
            // Add a comma if it's not the last field
            if field != table.table_schema.fields.last().unwrap() {
                scope_output.push_str(",\n");
                defaults_output.push_str(",\n");
            } else {
                scope_output.push_str("\n");
            }
        }

        scope_output.push_str("},\n");
        defaults_output.push_str("\n};\n");
        types_output.push_str(&format!(
            "export type {} = typeof bindings.{}.infer;\n",
            type_name, type_name
        ));
    }
    scope_output.push_str("\n});\n\n");

    if print_types {
        output.push_str(&format!(
            "{scope_output}\n{defaults_output}\n{types_output}"
        ));
    } else {
        output.push_str(&format!("{scope_output}\n{defaults_output}"));
    }

    output
}

fn field_type_to_arktype(
    field_type: &FieldType,
    tables: &[Schema],
    enums: &[TaggedUnion],
) -> String {
    match field_type {
        FieldType::String => "'string'".to_string(),
        FieldType::Char => "'string'".to_string(),
        FieldType::Bool => "'boolean'".to_string(),
        FieldType::Unit => "'null'".to_string(),
        FieldType::F32 | FieldType::F64 => "'number'".to_string(),
        FieldType::I8
        | FieldType::I16
        | FieldType::I32
        | FieldType::I64
        | FieldType::I128
        | FieldType::Isize => "'number'".to_string(),
        FieldType::U8
        | FieldType::U16
        | FieldType::U32
        | FieldType::U64
        | FieldType::U128
        | FieldType::Usize => "'number'".to_string(),
        FieldType::SpectaRecordId => r#""string""#.to_string(),

        FieldType::Tuple(types) => {
            let types_str = types
                .iter()
                .map(|t| field_type_to_arktype(t, tables, enums))
                .collect::<Vec<String>>()
                .join(", ");
            format!("[{}]", types_str)
        }

        FieldType::Struct(fields) => {
            let fields_str = fields
                .iter()
                .map(|(name, field_type)| {
                    format!(
                        "{}: {}",
                        name,
                        field_type_to_arktype(field_type, tables, enums)
                    )
                })
                .collect::<Vec<String>>()
                .join(", ");
            format!("{{ {} }}", fields_str)
        }

        FieldType::Option(inner) => {
            format!(
                "[[{}, '|', 'undefined'], '|', 'null']",
                field_type_to_arktype(inner, tables, enums)
            )
        }

        FieldType::Vec(inner) => {
            format!("[{}, '[]']", field_type_to_arktype(inner, tables, enums))
        }

        FieldType::HashMap(key, value) => {
            format!(
                "'Record<{}, {}>'",
                field_type_to_arktype(key, tables, enums).replace('\'', ""),
                field_type_to_arktype(value, tables, enums).replace('\'', "")
            )
        }
        FieldType::BTreeMap(key, value) => {
            format!(
                "'Record<{}, {}>'",
                field_type_to_arktype(key, tables, enums).replace('\'', ""),
                field_type_to_arktype(value, tables, enums).replace('\'', "")
            )
        }

        FieldType::RecordLink(inner) => format!(
            r#"[{}, "|",  "string"]"#,
            field_type_to_arktype(inner, tables, enums)
        ),

        FieldType::Other(type_name) => {
            // Try to find a matching table
            for table in tables {
                if table.table_schema.table_name == *type_name {
                    return to_pascal_case(&format!("'{}'", type_name));
                }
            }

            // Try to find a matching enum
            for schema_enum in enums {
                if schema_enum.enum_name == *type_name {
                    return to_pascal_case(&format!("'{}'", type_name));
                }
            }

            if let Some(enum_def) = enums.iter().find(|e| e.enum_name == *type_name) {
                let variants: Vec<String> = enum_def
                    .variants
                    .iter()
                    .map(|v| {
                        if v.data.is_some() {
                            format!(
                                "{}",
                                field_type_to_arktype(v.data.as_ref().unwrap(), tables, enums)
                            )
                        } else {
                            format!("'{}'", to_pascal_case(&v.name))
                        }
                    })
                    .collect();
                return variants.join(" | ");
            }

            // If no match found, return the type as is
            to_pascal_case(&format!("'{}'", type_name))
        }
    }
}

fn field_type_to_effect_schema(
    field_type: &FieldType,
    tables: &[Schema],
    enums: &[TaggedUnion],
    current: &str,       // NEW: name of the type we are expanding
    rec: &RecursionInfo, // NEW: recursion helper
    processed: &HashSet<String>,
) -> String {
    // helper to recurse with the same context
    let field = |inner: &FieldType| -> String {
        field_type_to_effect_schema(inner, tables, enums, current, rec, processed)
    };
    match field_type {
        FieldType::String => "Schema.String".to_string(),
        FieldType::Char => "Schema.String".to_string(),
        FieldType::Bool => "Schema.Boolean".to_string(),
        FieldType::Unit => "Schema.Null".to_string(),
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
        FieldType::SpectaRecordId => "Schema.String".to_string(),
        FieldType::Option(i) => format!("Schema.Option({})", field(i)),
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
            let pascal = to_pascal_case(name);
            // --- decide whether we need Schema.suspend --------------------
            if rec.is_recursive_pair(current, &pascal) && !processed.contains(&pascal) {
                // forward edge *inside* recursive SCC → suspend
                if tables
                    .iter()
                    .any(|t| to_pascal_case(&t.table_schema.table_name) == pascal)
                {
                    format!(
                        "Schema.suspend((): Schema.Schema<{}> => Schema.instanceOf({}))",
                        pascal, pascal
                    )
                } else {
                    format!(
                        "Schema.suspend((): Schema.Schema<typeof {}.Type> => {})",
                        pascal, pascal
                    )
                }
            } else {
                // everything else: direct reference
                pascal
            }
        }
    }
}

pub fn field_type_to_default_value(
    field_type: &FieldType,
    tables: &[Schema],
    enums: &[TaggedUnion],
) -> String {
    match field_type {
        FieldType::String | FieldType::Char => r#""""#.to_string(),
        FieldType::Bool => "false".to_string(),
        FieldType::Unit => "undefined".to_string(),
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
        | FieldType::Usize => "0".to_string(),
        FieldType::SpectaRecordId => "''".to_string(),
        FieldType::Tuple(inner_types) => {
            let tuple_defaults: Vec<String> = inner_types
                .iter()
                .map(|ty| field_type_to_default_value(ty, tables, enums))
                .collect();
            format!("[{}]", tuple_defaults.join(", "))
        }
        FieldType::Struct(fields) => {
            let fields_str = fields
                .iter()
                .map(|(name, ftype)| {
                    format!(
                        "{}: {}",
                        to_camel_case(&name),
                        field_type_to_default_value(ftype, tables, enums)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {} }}", fields_str)
        }
        FieldType::Option(_) => {
            // You can decide whether to produce `null` or `undefined` or something else.
            // For TypeScript, `null` is a more direct representation of "no value."
            "null".to_string()
        }
        FieldType::Vec(_) => {
            // Returns an empty array as the default
            // but recursively if you wanted an "example entry" you could do:
            //   format!("[{}]", field_type_to_default_value(inner, tables, enums))
            "[]".to_string()
        }
        FieldType::HashMap(_, _) => {
            // Return an empty object as default
            "{}".to_string()
        }

        FieldType::BTreeMap(_, _) => {
            // Return an empty object as default
            "{}".to_string()
        }

        FieldType::RecordLink(_) => {
            // Could produce "null" or "0" depending on your usage pattern.
            // We'll pick "null" for "unlinked".
            "''".to_string()
        }
        FieldType::Other(name) => {
            // 1) If this is an enum, pick a random variant.
            // 2) Otherwise if it matches a known table, produce a default object for that table.
            // 3) If neither, fall back to 'undefined'.

            // First check for an enum of this name
            if let Some(enum_schema) = enums.iter().find(|e| e.enum_name == *name) {
                let mut rng = rng();
                if let Some(chosen_variant) = enum_schema.variants.choose(&mut rng) {
                    // If the variant has data, generate a default for it.
                    if let Some(ref data_type) = chosen_variant.data {
                        let data_default = field_type_to_default_value(data_type, tables, enums);
                        return format!("{}", data_default);
                    } else {
                        // A variant without data
                        return format!("'{}'", chosen_variant.name);
                    }
                } else {
                    // If no variants, fallback to undefined
                    return "undefined".to_string();
                }
            }

            if let Some(table) = tables
                .iter()
                .find(|t| to_pascal_case(&t.table_schema.table_name) == to_pascal_case(&*name))
            {
                // We treat this similarly to a struct:
                let fields_str = table
                    .table_schema
                    .fields
                    .iter()
                    .map(|table_field| {
                        format!(
                            "{}: {}",
                            to_camel_case(&table_field.field_name),
                            field_type_to_default_value(&table_field.field_type, tables, enums)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {} }}", fields_str)
            } else {
                // Not an enum or known table
                "undefined".to_string()
            }
        }
    }
}

fn generate_random_date(within_next: i64) -> String {
    let now: DateTime<Utc> = Utc::now();
    let max_seconds = within_next * 24 * 60 * 60; // total seconds in m days
    let mut rng = rand::rng(); // Fixed: use thread_rng() instead of rng()

    // Generate a random offset in seconds from now
    let random_secs = rng.random_range(0..max_seconds); // Fixed: use random_range instead of random_range
    let mut random_date = now + Duration::seconds(random_secs);

    // Round down to the nearest 15-minute interval
    let minute = random_date.minute();
    let rounded_minute = (minute / 15) * 15;

    // Create a new date with the rounded minutes and 0 seconds
    random_date = random_date
        .with_minute(rounded_minute)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();

    // Format the date to ISO-8601 with no sub-second precision
    random_date.to_rfc3339_opts(SecondsFormat::Secs, true)
}
/// Generates `n` ISO‑8601 duration strings where each duration is between 1 and 4 hours
/// and is divisible by 15 minutes.
/// Allowed durations (in minutes): 60, 75, 90, …, 240.
/// ISO‑8601 durations are formatted as, for example:
///   - "PT1H" for 60 minutes,
///   - "PT1H15M" for 75 minutes.
fn generate_iso8601_durations(n: usize) -> Vec<String> {
    let mut rng = rand::rng();
    let mut durations = Vec::with_capacity(n);

    // Each duration is a multiple of 15 minutes.
    // 1 hour = 60 minutes => 60/15 = 4 and 4 hours = 240 minutes => 240/15 = 16.
    // Thus, allowed multiplier values are between 4 and 16 (inclusive).
    for _ in 0..n {
        let k = rng.random_range(4..=16); // Random integer between 4 and 16.
        let total_minutes = k * 15;
        let hours = total_minutes / 60;
        let minutes = total_minutes % 60;

        // Format the duration as an ISO‑8601 duration string.
        let duration_str = if minutes == 0 {
            format!("PT{}H", hours)
        } else {
            format!("PT{}H{}M", hours, minutes)
        };
        durations.push(duration_str);
    }

    durations
}

/// Programmatically generates or retrieves hexadecimal codes for colors
/// equivalent to Tailwind's 300, 400, and 500 levels
///
/// # Arguments
///
/// * `color_name` - Optional color name. If None, a random color will be generated
///
/// # Returns
///
/// A tuple with three hex color strings for the 300, 400, and 500 levels respectively
pub fn generate_hex_colors(color_name: Option<&str>) -> Colors {
    match color_name {
        Some(name) => {
            // Try to get from predefined colors first
            if let Some(colors) = get_predefined_color(name) {
                return colors;
            }

            // If not found in predefined colors, generate based on the name (use as seed)
            generate_color_from_name(name)
        }
        None => {
            // Generate a random color
            generate_random_color()
        }
    }
}

/// Retrieves predefined Tailwind colors if they exist
fn get_predefined_color(color_name: &str) -> Option<Colors> {
    // Define some common Tailwind colors
    let mut color_map: HashMap<&str, Colors> = HashMap::new();

    // Just a subset of common colors for reference
    color_map.insert(
        "red",
        Colors {
            main: "#fca5a5".to_string(),
            hover: "#f87171".to_string(),
            active: "#ef4444".to_string(),
        },
    );
    color_map.insert(
        "blue",
        Colors {
            main: "#93c5fd".to_string(),
            hover: "#60a5fa".to_string(),
            active: "#3b82f6".to_string(),
        },
    );
    color_map.insert(
        "green",
        Colors {
            main: "#86efac".to_string(),
            hover: "#4ade80".to_string(),
            active: "#22c55e".to_string(),
        },
    );
    color_map.insert(
        "yellow",
        Colors {
            main: "#fde047".to_string(),
            hover: "#facc15".to_string(),
            active: "#eab308".to_string(),
        },
    );
    color_map.insert(
        "purple",
        Colors {
            main: "#d8b4fe".to_string(),
            hover: "#c084fc".to_string(),
            active: "#a855f7".to_string(),
        },
    );

    color_map.get(color_name.to_lowercase().as_str()).cloned()
}

/// Generates a color based on the provided name as a seed
fn generate_color_from_name(name: &str) -> Colors {
    // Use the name as a seed for generating a base hue
    let mut seed: u32 = 0;
    for c in name.chars() {
        seed = seed.wrapping_add(c as u32);
    }

    // Use the seed to generate a hue (0-360)
    let hue = seed % 360;

    // Generate the three colors based on this hue
    generate_color_from_hue(hue)
}

/// Generates a random color
fn generate_random_color() -> Colors {
    let mut rng = rand::rng();

    // Choose either a random predefined color or generate a new one
    let predefined_colors = [
        "red", "blue", "green", "yellow", "purple", "orange", "teal", "indigo", "pink", "gray",
    ];

    if rng.random_bool(0.3) {
        // 30% chance to use a predefined color
        if let Some(color) = predefined_colors.choose(&mut rng) {
            if let Some(colors) = get_predefined_color(color) {
                return colors;
            }
        }
    }

    // Generate a random hue (0-360)
    let hue = rng.random_range(0..360);
    generate_color_from_hue(hue)
}

/// Generates colors from a hue value using HSL color model
fn generate_color_from_hue(hue: u32) -> Colors {
    // Generate the three colors with different lightness and saturation values
    // to match the Tailwind 300, 400, 500 progression

    // Tailwind generally decreases lightness and sometimes adjusts saturation
    // as the number increases
    let color_300 = hsl_to_hex(hue, 85, 80); // Most light, high saturation
    let color_400 = hsl_to_hex(hue, 90, 65); // Medium
    let color_500 = hsl_to_hex(hue, 95, 50); // Darkest

    Colors {
        main: color_300,
        hover: color_400,
        active: color_500,
    }
}

/// Converts HSL (Hue, Saturation, Lightness) to Hex color
fn hsl_to_hex(h: u32, s: u32, l: u32) -> String {
    // Ensure proper ranges
    let h = h % 360;
    let s = s.min(100);
    let l = l.min(100);

    // Convert to 0-1 range
    let h_f = h as f64 / 360.0;
    let s_f = s as f64 / 100.0;
    let l_f = l as f64 / 100.0;

    // Algorithm to convert HSL to RGB
    let c = (1.0 - (2.0 * l_f - 1.0).abs()) * s_f;
    let x = c * (1.0 - ((h_f * 6.0) % 2.0 - 1.0).abs());
    let m = l_f - c / 2.0;

    let (r_1, g_1, b_1) = match (h_f * 6.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    // Convert to 0-255 range
    let r = ((r_1 + m) * 255.0).round() as u8;
    let g = ((g_1 + m) * 255.0).round() as u8;
    let b = ((b_1 + m) * 255.0).round() as u8;

    // Convert to hex
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

/// Generates a vector of n items using the provided generator function
///
/// # Arguments
///
/// * `n` - The number of items to generate
/// * `generator` - A function that generates each value
///
/// # Returns
///
/// A vector containing n generated items
///
/// # Examples
///
/// ```
/// // Generate a vector of 5 random numbers
/// let random_numbers = generate_vec(5, || rand::random::<i32>());
///
/// // Generate a vector of 3 incrementing values
/// let mut counter = 0;
/// let incremental = generate_vec(3, || {
///     counter += 1;
///     counter
/// });
/// assert_eq!(incremental, vec![1, 2, 3]);
/// ```
pub fn generate_vec<T, F>(n: usize, mut generator: F) -> Vec<T>
where
    F: FnMut() -> T,
{
    let mut result = Vec::with_capacity(n);
    for _ in 0..n {
        result.push(generator());
    }
    result
}
fn convert_type_declaration(input: &str) -> String {
    // Updated pattern to handle both semicolon-terminated and end-of-object properties
    let property_pattern = Regex::new(r"(\w+)_(\w+):\s*([^;}\]]+(?:\[\])?)[;]?").unwrap();

    let mut result = input.to_string();
    let mut offset = 0;

    while let Some(cap) = property_pattern.find_at(&result, offset) {
        let matched = cap.as_str();
        let start = cap.start();
        let end = cap.end();

        let colon_pos = matched.find(':').unwrap();
        let snake_case = &matched[..colon_pos].trim();
        let type_part = &matched[colon_pos..];

        let camel_case = to_camel_case(snake_case);
        let replacement = format!("{}{}", camel_case, type_part);

        result.replace_range(start..end, &replacement);
        offset = start + replacement.len();
    }

    result
}

fn convert_type_declarations(declarations: Vec<String>) -> Vec<String> {
    declarations
        .into_iter()
        .map(|decl| convert_type_declaration(&decl))
        .collect()
}
fn collect_specta_types() -> Vec<String> {
    convert_type_declarations(vec![
        ts::export::<Order>(&ExportConfiguration::default()).unwrap(),
        ts::export::<BilledItem>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Account>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Appointment>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Colors>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Employee>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Email>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Lead>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Route>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Service>(&ExportConfiguration::default()).unwrap(),
        ts::export::<ServiceDefaults>(&ExportConfiguration::default()).unwrap(),
        ts::export::<TaxRate>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Color>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Coordinates>(&ExportConfiguration::default()).unwrap(),
        ts::export::<PhoneNumber>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Sector>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Site>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Priority>(&ExportConfiguration::default()).unwrap(),
        ts::export::<LeadStage>(&ExportConfiguration::default()).unwrap(),
        ts::export::<NextStep>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Settings>(&ExportConfiguration::default()).unwrap(),
        ts::export::<AppointmentNotifications>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Commissions>(&ExportConfiguration::default()).unwrap(),
        ts::export::<User>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Metadata>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Company>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Product>(&ExportConfiguration::default()).unwrap(),
        ts::export::<ProductDefaults>(&ExportConfiguration::default()).unwrap(),
        ts::export::<OrderStage>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Payment>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Item>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Promotion>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Package>(&ExportConfiguration::default()).unwrap(),
        ts::export::<RecordLink<Value>>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Ordered>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Represents>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Status>(&ExportConfiguration::default()).unwrap(),
        ts::export::<RecurrenceRule>(&ExportConfiguration::default()).unwrap(),
        ts::export::<MonthlyRecurrenceRule>(&ExportConfiguration::default()).unwrap(),
        ts::export::<WeeklyRecurrenceRule>(&ExportConfiguration::default()).unwrap(),
        ts::export::<DailyRecurrenceRule>(&ExportConfiguration::default()).unwrap(),
        ts::export::<YearlyRecurrenceRule>(&ExportConfiguration::default()).unwrap(),
        ts::export::<WeekOfMonth>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Weekday>(&ExportConfiguration::default()).unwrap(),
        ts::export::<RecurrenceEnd>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Interval>(&ExportConfiguration::default()).unwrap(),
        ts::export::<AppPermissions>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Applications>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Table>(&ExportConfiguration::default()).unwrap(),
        ts::export::<Page>(&ExportConfiguration::default()).unwrap(),
        ts::export::<UserRole>(&ExportConfiguration::default()).unwrap(),
    ])
}

pub fn generate_random_recurrence_rule_json() -> String {
    let mut rng = rng();

    // Randomly choose which interval variant to create
    let interval = match rng.random_range(0..4) {
        0 => Interval::Daily(DailyRecurrenceRule {
            quantity_of_days: rng.random_range(1..31),
        }),
        1 => Interval::Weekly(WeeklyRecurrenceRule {
            quantity_of_weeks: rng.random_range(1..5),
            weekdays: random_weekdays(&mut rng),
        }),
        2 => Interval::Monthly(MonthlyRecurrenceRule {
            quantity_of_months: rng.random_range(1..12),
            // Choose any day up to 28 for safety
            day: rng.random_range(1..29),
            name: random_string(5, &mut rng),
        }),
        _ => Interval::Yearly(YearlyRecurrenceRule {
            quantity_of_years: rng.random_range(1..5),
        }),
    };

    let recurrence_begins = generate_random_date(7);

    // Randomly create an end to the recurrence
    let recurrence_ends = if rng.random_bool(0.5) {
        // Half of the time produce After, half On
        if rng.random_bool(0.5) {
            Some(RecurrenceEnd::After(rng.random_range(1..=20)))
        } else {
            Some(RecurrenceEnd::On(generate_random_date(90)))
        }
    } else {
        None
    };

    // Generate random additional instances (30% chance)
    let _additional_instances = if rng.random_bool(0.3) {
        let count = rng.random_range(1..3);
        let mut instances = Vec::with_capacity(count);
        for _ in 0..count {
            let random_id = random_string(8, &mut rng);
            instances.push(Appointment {
                id: format!("appointment:{}", &random_id).into(),
                title: format!("Additional Instance {}", random_string(5, &mut rng)),
                status: Status::Scheduled,
                begins: generate_random_date(30),
                duration: format!("PT{}H", rng.random_range(1..4)),
                timezone: "America/Los_Angeles".to_string(),
                offset_ms: 0,
                all_day: false,
                multi_day: false,
                employees: vec![],
                location: RecordLink::Id(format!("site:{}", &random_string(8, &mut rng)).into()),
                description: Some(random_string(20, &mut rng)),
                colors: Colors {
                    main: "#93c5fd".to_string(),
                    hover: "#60a5fa".to_string(),
                    active: "#3b82f6".to_string(),
                },
                recurrence_rule: None,
            });
        }
        Some(instances)
    } else {
        None
    };

    // Create exceptions with optional cancelled instances
    let cancelled_instances = if rng.random_bool(0.5) {
        // Just produce between 1-3 random datetimes
        let count = rng.random_range(1..4);
        let mut instances = Vec::with_capacity(count);
        for _ in 0..count {
            instances.push(format!(
                "20{:02}-{:02}-{:02}T00:00:00Z",
                rng.random_range(23..30), // e.g., 2023-2029
                rng.random_range(1..13),
                rng.random_range(1..29)
            ));
        }
        Some(instances)
    } else {
        None
    };

    let rule = RecurrenceRule {
        interval,
        recurrence_begins,
        recurrence_ends,
        cancelled_instances,
    };

    // Convert to pretty-printed JSON
    serde_json::to_string_pretty(&rule).unwrap()
}

// Helper to generate a random string of a given length
fn random_string(length: usize, rng: &mut impl Rng) -> String {
    (0..length)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}

// Helper to randomly choose a subset of weekdays
fn random_weekdays(rng: &mut impl Rng) -> Vec<Weekday> {
    // Potential weekdays
    let all_days = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];

    // Possibly select each weekday with 50% probability
    all_days
        .iter()
        .filter(|_| rng.random_bool(0.5))
        .cloned()
        .collect()
}
