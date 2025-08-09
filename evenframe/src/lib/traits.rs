use crate::schemasync::TableConfig;

/// Trait for persistable structs (with ID field, representing database tables)
pub trait EvenframePersistableStruct {
    // Keep table_config for runtime CRUD operations
    fn table_config() -> Option<TableConfig> {
        None
    }
}

use serde::Deserializer;

pub trait EvenframeDeserialize<'de>: Sized {
    fn evenframe_deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}
