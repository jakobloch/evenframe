use quote::quote;

/// Generate imports for struct trait implementations
pub fn generate_struct_trait_imports() -> proc_macro2::TokenStream {
    quote! {
        use evenframe::{
            traits::EvenframePersistableStruct,
            types::{StructConfig, StructField},
            validator::{StringValidator, Validator},
        };
    }
}

/// Generate imports for table configuration in persistable structs
pub fn generate_table_config_imports() -> proc_macro2::TokenStream {
    quote! {
        use evenframe::{
            config::EvenframeConfig,
            schemasync::{
                mockmake::MockGenerationConfig,
                compare::PreservationMode,
                TableConfig,
            },
        };
        use convert_case::{Case, Casing};
    }
}

/// Generate imports for parsing struct attributes
pub fn generate_struct_parsing_imports() -> proc_macro2::TokenStream {
    quote! {
        use evenframe::{
            schemasync::{
                DefineConfig, Direction, EdgeConfig, PermissionsConfig,
            },
        };
    }
}

/// Generate imports for enum trait implementation (no longer needed - enums don't generate code)
pub fn generate_enum_trait_imports() -> proc_macro2::TokenStream {
    quote! {}
}

/// Generate imports needed for deserialization
pub fn generate_deserialize_imports() -> proc_macro2::TokenStream {
    quote! {
        use evenframe::traits::EvenframeDeserialize;
    }
}

/// Generate combined imports for struct implementations
pub fn generate_struct_imports() -> proc_macro2::TokenStream {
    let trait_imports = generate_struct_trait_imports();
    let table_imports = generate_table_config_imports();
    let parsing_imports = generate_struct_parsing_imports();

    quote! {
        #trait_imports
        #table_imports
        #parsing_imports
    }
}

/// Generate all imports needed for enum implementations
pub fn generate_enum_imports() -> proc_macro2::TokenStream {
    generate_enum_trait_imports()
}
