use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput};
use tracing::{debug, error, info};

pub fn generate_enum_impl(input: DeriveInput) -> TokenStream {
    let ident = input.ident;
    info!("Generating enum implementation for: {}", ident);

    if let Data::Enum(ref _data_enum) = input.data {
        debug!("Processing enum data for: {}", ident);
        // No code generation needed - the derive macro itself serves as the marker
        // Config is parsed directly from source by config_builders.rs
        info!(
            "Successfully processed enum: {} (no code generation needed)",
            ident
        );
        quote! {}
    } else {
        error!(
            "Attempted to use generate_enum_impl on non-enum type: {}",
            ident
        );
        syn::Error::new(
            ident.span(),
            format!("The Evenframe derive macro can only be applied to enums when using generate_enum_impl.\n\nYou tried to apply it to: {}\n\nExample of correct usage:\n#[derive(Evenframe)]\nenum MyEnum {{\n    Variant1,\n    Variant2(String),\n    Variant3 {{ field: i32 }}\n}}", ident),
        )
        .to_compile_error()
    }
}
