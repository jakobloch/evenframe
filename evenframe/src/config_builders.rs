use std::collections::HashMap;
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
use helpers::evenframe::{
    schemasync::table::TableConfig,
    traits::{EvenframeAppStruct, EvenframeEnum, EvenframePersistableStruct},
    types::{StructConfig, TaggedUnion},
};
use helpers::case::to_snake_case;

pub fn build_all_configs() -> impl FnOnce() -> (
    HashMap<String, TaggedUnion>,   // enum_configs
    HashMap<String, TableConfig>,   // tables
    HashMap<String, StructConfig>,  // objects
) {
    || {
        (
            build_enum_configs(),
            build_tables(),
            build_objects(),
        )
    }
}

pub fn merge_tables_and_objects(
    tables: &HashMap<String, TableConfig>,
    objects: &HashMap<String, StructConfig>,
) -> HashMap<String, StructConfig> {
    let mut struct_configs = objects.clone();
    
    // Extract StructConfig from each TableConfig and merge into struct_configs
    for (name, table_config) in tables {
        struct_configs.insert(name.clone(), table_config.struct_config.clone());
    }
    
    struct_configs
}


fn build_enum_configs() -> HashMap<String, TaggedUnion> {
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
    
    enum_configs
}

fn build_tables() -> HashMap<String, TableConfig> {
    let mut tables = HashMap::new();

    macro_rules! insert_table_config {
        ($type:ty) => {
            if let Some(table_config) = <$type as EvenframePersistableStruct>::table_config() {
                tables.insert(
                    to_snake_case(&<$type as EvenframePersistableStruct>::name()),
                    table_config,
                );
            }
        };
    }

    insert_table_config!(Account);
    insert_table_config!(Appointment);
    insert_table_config!(Lead);
    insert_table_config!(TaxRate);
    insert_table_config!(Site);
    insert_table_config!(Employee);
    insert_table_config!(Route);
    insert_table_config!(Company);
    insert_table_config!(Product);
    insert_table_config!(Service);
    insert_table_config!(User);
    insert_table_config!(Order);
    insert_table_config!(Payment);
    insert_table_config!(Package);
    insert_table_config!(Promotion);
    insert_table_config!(Represents);
    insert_table_config!(Ordered);

    tables
}

fn build_objects() -> HashMap<String, StructConfig> {
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

    objects
}

