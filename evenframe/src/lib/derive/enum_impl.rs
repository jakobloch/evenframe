use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput};

pub fn generate_enum_impl(input: DeriveInput) -> TokenStream {
    let ident = input.ident;
    
    if let Data::Enum(ref _data_enum) = input.data {
        // No code generation needed - the derive macro itself serves as the marker
        // Config is parsed directly from source by config_builders.rs
        quote! {}.into()
    } else {
        syn::Error::new(
            ident.span(),
            format!("The Evenframe derive macro can only be applied to enums when using generate_enum_impl.\n\nYou tried to apply it to: {}\n\nExample of correct usage:\n#[derive(Evenframe)]\nenum MyEnum {{\n    Variant1,\n    Variant2(String),\n    Variant3 {{ field: i32 }}\n}}", ident),
        )
        .to_compile_error()
        .into()
    }
}