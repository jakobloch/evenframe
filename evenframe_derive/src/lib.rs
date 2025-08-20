use evenframe_core::derive::{enum_impl, struct_impl};
use proc_macro::TokenStream;
use syn::{Data, DeriveInput, parse_macro_input};

/// For structs it generates both:
/// - A `table_schema()` function returning a `helpers::TableSchema`
#[proc_macro_derive(
    Evenframe,
    attributes(
        edge,
        define_field_statement,
        format,
        permissions,
        mock_data,
        validators,
        relation
    )
)]
pub fn evenframe_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match input.data {
        Data::Struct(_) => struct_impl::generate_struct_impl(input).into(),
        Data::Enum(_) => enum_impl::generate_enum_impl(input).into(),
        _ => syn::Error::new(
            input.ident.span(),
            "Evenframe can only be used on structs and enums",
        )
        .to_compile_error()
        .into(),
    }
}
