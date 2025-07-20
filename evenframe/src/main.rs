use dotenv::dotenv;
use handlers::{
    account::{Account, AccountName, Ordered},
    appointment::{
        Appointment, Colors, Interval, RecurrenceEnd, RecurrenceRule, Status, WeekOfMonth, Weekday,
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
    validation::account_validation::{Color, Coordinates, PhoneNumber, Sector, Site},
    Email,
};
use helpers::case::to_snake_case;
use helpers::evenframe::{
    config::EvenframeConfig,
    schemasync::*,
    traits::{EvenframeAppStruct, EvenframeEnum, EvenframePersistableStruct},
    typesync::{arktype::generate_arktype_type_string, effect::generate_effect_schema_string},
};
use std::{collections::HashMap, env, fs, path::Path};
use surrealdb::{engine::remote::http::Http, opt::auth::Root, Surreal};
use toml;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok(); // Load .env file

    // Load configuration
    let config = load_config()?;

    let generate_dummy_values = config.schemasync.should_generate_mocks;
    let generate_arktype_types = config.typesync.should_generate_arktype_types;
    let generate_effect_schemas = config.typesync.should_generate_effect_types;

    // Create shared HashMap for all struct configs
    let mut struct_configs = HashMap::new();

    // Helper macros to insert struct configs
    macro_rules! insert_app_struct_config {
        ($type:ty) => {
            struct_configs.insert(
                to_snake_case(&<$type as EvenframeAppStruct>::name()),
                <$type as EvenframeAppStruct>::struct_config(),
            );
        };
    }

    macro_rules! insert_persistable_struct_config {
        ($type:ty) => {
            struct_configs.insert(
                to_snake_case(&<$type as EvenframePersistableStruct>::name()),
                <$type as EvenframePersistableStruct>::struct_config(),
            );
        };
    }

    // Insert all struct configs (both tables and non-tables)
    // Non-persistable structs (no ID field)
    insert_app_struct_config!(PhoneNumber);
    insert_app_struct_config!(Coordinates);
    insert_app_struct_config!(Commissions);
    insert_app_struct_config!(Settings);
    insert_app_struct_config!(Colors);
    insert_app_struct_config!(Email);
    insert_app_struct_config!(BilledItem);
    insert_app_struct_config!(AppointmentNotifications);
    insert_app_struct_config!(Metadata);
    insert_app_struct_config!(Color);
    insert_app_struct_config!(ServiceDefaults);
    insert_app_struct_config!(ProductDefaults);
    insert_app_struct_config!(RecurrenceRule);
    insert_app_struct_config!(AppPermissions);

    // Persistable structs (with ID field)
    insert_persistable_struct_config!(Account);
    insert_persistable_struct_config!(Appointment);
    insert_persistable_struct_config!(Lead);
    insert_persistable_struct_config!(TaxRate);
    insert_persistable_struct_config!(Site);
    insert_persistable_struct_config!(Employee);
    insert_persistable_struct_config!(Route);
    insert_persistable_struct_config!(Company);
    insert_persistable_struct_config!(Product);
    insert_persistable_struct_config!(Service);
    insert_persistable_struct_config!(User);
    insert_persistable_struct_config!(Order);
    insert_persistable_struct_config!(Payment);
    insert_persistable_struct_config!(Package);
    insert_persistable_struct_config!(Promotion);
    insert_persistable_struct_config!(Represents);
    insert_persistable_struct_config!(Ordered);

    // Create HashMap for enums
    let mut enum_configs = HashMap::new();
    enum_configs.insert(Sector::name(), Sector::tagged_union());
    enum_configs.insert(LeadStage::name(), LeadStage::tagged_union());
    enum_configs.insert(NextStep::name(), NextStep::tagged_union());
    enum_configs.insert(Priority::name(), Priority::tagged_union());
    enum_configs.insert(OrderStage::name(), OrderStage::tagged_union());
    enum_configs.insert(Item::name(), Item::tagged_union());
    enum_configs.insert(Status::name(), Status::tagged_union());
    enum_configs.insert(WeekOfMonth::name(), WeekOfMonth::tagged_union());
    enum_configs.insert(Weekday::name(), Weekday::tagged_union());
    enum_configs.insert(RecurrenceEnd::name(), RecurrenceEnd::tagged_union());
    enum_configs.insert(Interval::name(), Interval::tagged_union());
    enum_configs.insert(Table::name(), Table::tagged_union());
    enum_configs.insert(Page::name(), Page::tagged_union());
    enum_configs.insert(Applications::name(), Applications::tagged_union());
    enum_configs.insert(UserRole::name(), UserRole::tagged_union());
    enum_configs.insert(AccountName::name(), AccountName::tagged_union());

    // Merge inline structs from enums into struct_configs for type generation
    macro_rules! merge_enum_inline_structs_to_configs {
        ($enum_type:ty) => {
            if let Some(inline_structs) = <$enum_type as EvenframeEnum>::inline_structs() {
                for inline_struct in inline_structs {
                    struct_configs.insert(
                        to_snake_case(&inline_struct.name),
                        inline_struct,
                    );
                }
            }
        };
    }

    // Merge inline structs from all enums
    merge_enum_inline_structs_to_configs!(Sector);
    merge_enum_inline_structs_to_configs!(LeadStage);
    merge_enum_inline_structs_to_configs!(NextStep);
    merge_enum_inline_structs_to_configs!(Priority);
    merge_enum_inline_structs_to_configs!(OrderStage);
    merge_enum_inline_structs_to_configs!(Item);
    merge_enum_inline_structs_to_configs!(Status);
    merge_enum_inline_structs_to_configs!(WeekOfMonth);
    merge_enum_inline_structs_to_configs!(Weekday);
    merge_enum_inline_structs_to_configs!(RecurrenceEnd);
    merge_enum_inline_structs_to_configs!(Interval);
    merge_enum_inline_structs_to_configs!(Table);
    merge_enum_inline_structs_to_configs!(Page);
    merge_enum_inline_structs_to_configs!(Applications);
    merge_enum_inline_structs_to_configs!(UserRole);
    merge_enum_inline_structs_to_configs!(AccountName);

    if generate_arktype_types {
        std::fs::write(
            "../../frontend/src/lib/core/types/arktype.ts",
            format!(
                "import {{ scope }} from 'arktype';\n\n{}\n\n export const validator = scope({{
  ...bindings.export(),
            }}).export();",
                generate_arktype_type_string(&struct_configs, &enum_configs, false),
            ),
        )?;
    }

    if generate_effect_schemas {
        std::fs::write(
            "../../frontend/src/lib/core/types/bindings.ts",
            format!(
                "import {{ Schema }} from \"effect\";\n\n{}",
                generate_effect_schema_string(&struct_configs, &enum_configs, false),
            ),
        )?;
    }

    if generate_dummy_values {
        match Surreal::new::<Http>(&config.schemasync.database.url).await {
            Ok(db) => {
                // Try to get credentials from env vars, otherwise use defaults
                let username = env::var("SURREAL_USER").unwrap_or_else(|_| "root".to_string());
                let password = env::var("SURREAL_PASSWORD").unwrap_or_else(|_| "root".to_string());

                db.signin(Root {
                    username: &username,
                    password: &password,
                })
                .await?;

                db.use_ns(&config.schemasync.database.namespace)
                    .use_db(&config.schemasync.database.database)
                    .await?;
                // Create a HashMap of TableConfig items
                let mut tables = HashMap::new();

                if let Some(table_config) = <Account as EvenframePersistableStruct>::table_config()
                {
                    tables.insert(
                        to_snake_case(&<Account as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) =
                    <Appointment as EvenframePersistableStruct>::table_config()
                {
                    tables.insert(
                        to_snake_case(&<Appointment as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) = <Lead as EvenframePersistableStruct>::table_config() {
                    tables.insert(
                        to_snake_case(&<Lead as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) = <TaxRate as EvenframePersistableStruct>::table_config()
                {
                    tables.insert(
                        to_snake_case(&<TaxRate as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) = <Site as EvenframePersistableStruct>::table_config() {
                    tables.insert(
                        to_snake_case(&<Site as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) = <Employee as EvenframePersistableStruct>::table_config()
                {
                    tables.insert(
                        to_snake_case(&<Employee as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) = <Route as EvenframePersistableStruct>::table_config() {
                    tables.insert(
                        to_snake_case(&<Route as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) = <Company as EvenframePersistableStruct>::table_config()
                {
                    tables.insert(
                        to_snake_case(&<Company as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) = <Product as EvenframePersistableStruct>::table_config()
                {
                    tables.insert(
                        to_snake_case(&<Product as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) = <Service as EvenframePersistableStruct>::table_config()
                {
                    tables.insert(
                        to_snake_case(&<Service as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) = <User as EvenframePersistableStruct>::table_config() {
                    tables.insert(
                        to_snake_case(&<User as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) = <Order as EvenframePersistableStruct>::table_config() {
                    tables.insert(
                        to_snake_case(&<Order as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) = <Payment as EvenframePersistableStruct>::table_config()
                {
                    tables.insert(
                        to_snake_case(&<Payment as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) = <Package as EvenframePersistableStruct>::table_config()
                {
                    tables.insert(
                        to_snake_case(&<Package as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) =
                    <Promotion as EvenframePersistableStruct>::table_config()
                {
                    tables.insert(
                        to_snake_case(&<Promotion as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) =
                    <Represents as EvenframePersistableStruct>::table_config()
                {
                    tables.insert(
                        to_snake_case(&<Represents as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }
                if let Some(table_config) = <Ordered as EvenframePersistableStruct>::table_config()
                {
                    tables.insert(
                        to_snake_case(&<Ordered as EvenframePersistableStruct>::name()),
                        table_config,
                    );
                }

                // Create a HashMap of StructConfig items
                let mut objects = HashMap::new();

                objects.insert(
                    <PhoneNumber as EvenframeAppStruct>::name(),
                    <PhoneNumber as EvenframeAppStruct>::struct_config(),
                );
                objects.insert(
                    <Coordinates as EvenframeAppStruct>::name(),
                    <Coordinates as EvenframeAppStruct>::struct_config(),
                );
                objects.insert(
                    <Settings as EvenframeAppStruct>::name(),
                    <Settings as EvenframeAppStruct>::struct_config(),
                );
                objects.insert(
                    <Colors as EvenframeAppStruct>::name(),
                    <Colors as EvenframeAppStruct>::struct_config(),
                );
                objects.insert(
                    <Email as EvenframeAppStruct>::name(),
                    <Email as EvenframeAppStruct>::struct_config(),
                );
                objects.insert(
                    <BilledItem as EvenframeAppStruct>::name(),
                    <BilledItem as EvenframeAppStruct>::struct_config(),
                );
                objects.insert(
                    <AppointmentNotifications as EvenframeAppStruct>::name(),
                    <AppointmentNotifications as EvenframeAppStruct>::struct_config(),
                );
                objects.insert(
                    <Commissions as EvenframeAppStruct>::name(),
                    <Commissions as EvenframeAppStruct>::struct_config(),
                );
                objects.insert(
                    <Metadata as EvenframeAppStruct>::name(),
                    <Metadata as EvenframeAppStruct>::struct_config(),
                );
                objects.insert(
                    <Color as EvenframeAppStruct>::name(),
                    <Color as EvenframeAppStruct>::struct_config(),
                );
                objects.insert(
                    <ServiceDefaults as EvenframeAppStruct>::name(),
                    <ServiceDefaults as EvenframeAppStruct>::struct_config(),
                );
                objects.insert(
                    <ProductDefaults as EvenframeAppStruct>::name(),
                    <ProductDefaults as EvenframeAppStruct>::struct_config(),
                );
                objects.insert(
                    <RecurrenceRule as EvenframeAppStruct>::name(),
                    <RecurrenceRule as EvenframeAppStruct>::struct_config(),
                );
                objects.insert(
                    <AppPermissions as EvenframeAppStruct>::name(),
                    <AppPermissions as EvenframeAppStruct>::struct_config(),
                );

                // Create a HashMap of TaggedUnion items for enums
                let mut enums = HashMap::new();

                enums.insert(Sector::name(), Sector::tagged_union());
                enums.insert(LeadStage::name(), LeadStage::tagged_union());
                enums.insert(NextStep::name(), NextStep::tagged_union());
                enums.insert(Priority::name(), Priority::tagged_union());
                enums.insert(OrderStage::name(), OrderStage::tagged_union());
                enums.insert(Item::name(), Item::tagged_union());
                enums.insert(Status::name(), Status::tagged_union());
                enums.insert(WeekOfMonth::name(), WeekOfMonth::tagged_union());
                enums.insert(Weekday::name(), Weekday::tagged_union());
                enums.insert(RecurrenceEnd::name(), RecurrenceEnd::tagged_union());
                enums.insert(Interval::name(), Interval::tagged_union());
                enums.insert(Table::name(), Table::tagged_union());
                enums.insert(Page::name(), Page::tagged_union());
                enums.insert(Applications::name(), Applications::tagged_union());
                enums.insert(UserRole::name(), UserRole::tagged_union());
                enums.insert(AccountName::name(), AccountName::tagged_union());

                // Merge inline structs from enums into objects
                macro_rules! merge_enum_inline_structs {
                    ($enum_type:ty) => {
                        if let Some(inline_structs) = <$enum_type as EvenframeEnum>::inline_structs() {
                            for inline_struct in inline_structs {
                                objects.insert(inline_struct.name.clone(), inline_struct);
                            }
                        }
                    };
                }

                // Merge inline structs from all enums
                merge_enum_inline_structs!(Sector);
                merge_enum_inline_structs!(LeadStage);
                merge_enum_inline_structs!(NextStep);
                merge_enum_inline_structs!(Priority);
                merge_enum_inline_structs!(OrderStage);
                merge_enum_inline_structs!(Item);
                merge_enum_inline_structs!(Status);
                merge_enum_inline_structs!(WeekOfMonth);
                merge_enum_inline_structs!(Weekday);
                merge_enum_inline_structs!(RecurrenceEnd);
                merge_enum_inline_structs!(Interval);
                merge_enum_inline_structs!(Table);
                merge_enum_inline_structs!(Page);
                merge_enum_inline_structs!(Applications);
                merge_enum_inline_structs!(UserRole);
                merge_enum_inline_structs!(AccountName);

                run_schemasync(db, &tables, &objects, &enums, config.schemasync.clone()).await?;
            }
            Err(e) => {
                eprintln!("Database connection error: {}", e);
                return Err(e.into());
            }
        }
    }

    Ok(())
}

/// Load configuration from evenframe.toml
fn load_config() -> Result<EvenframeConfig, Box<dyn std::error::Error>> {
    // Try to find evenframe.toml in the backend directory
    let config_path = Path::new("../evenframe.toml");

    if !config_path.exists() {
        return Err("evenframe.toml not found. Configuration file is required.".into());
    }

    let contents = fs::read_to_string(config_path)?;
    let mut config: EvenframeConfig = toml::from_str(&contents)?;

    // Process environment variable substitutions for all database-related fields
    config.schemasync.database.url = substitute_env_vars(&config.schemasync.database.url);
    config.schemasync.database.namespace =
        substitute_env_vars(&config.schemasync.database.namespace);
    config.schemasync.database.database = substitute_env_vars(&config.schemasync.database.database);

    Ok(config)
}

/// Substitute environment variables in config strings
/// Supports ${VAR_NAME:-default} syntax
fn substitute_env_vars(value: &str) -> String {
    let mut result = value.to_string();

    // Pattern to match ${VAR_NAME} or ${VAR_NAME:-default}
    let re = regex::Regex::new(r"\$\{([^}:]+)(?::-([^}]*))?\}").unwrap();

    for cap in re.captures_iter(value) {
        let var_name = &cap[1];
        let default_value = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        let replacement = env::var(var_name).unwrap_or_else(|_| default_value.to_string());
        let full_match = &cap[0];
        result = result.replace(full_match, &replacement);
    }

    result
}
