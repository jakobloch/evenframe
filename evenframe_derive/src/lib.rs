use evenframe_core::derive::{enum_impl, struct_impl};
use proc_macro::TokenStream;
use syn::{Data, DeriveInput, parse_macro_input};

/// For structs it generates both:
/// - A `table_schema()` function returning a `helpers::TableSchema`, and
/// - CRUD async functions (`create`, `update`, `delete`, `read`, `fetch`)
///   that build JSON payloads and generate SQL query strings including:
///     - For fields with an `edge` attribute, subqueries in the SELECT clause.
///     - For fields with a `fetch` attribute, a FETCH clause listing the fetched field names.
///   If the struct does not contain an "id" field, the handler functions are omitted.
/// For enums it generates a `variants()` method returning a `TaggedUnion`.
#[proc_macro_derive(
    Evenframe,
    attributes(
        edge,
        fetch,
        define_field_statement,
        subquery,
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
