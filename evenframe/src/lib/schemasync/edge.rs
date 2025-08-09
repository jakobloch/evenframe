use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::fmt;
use std::str::FromStr;
use syn::parenthesized;
use syn::spanned::Spanned;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EdgeConfig {
    pub edge_name: String,
    pub from: String,
    pub to: String,
    pub direction: Direction,
}

impl ToTokens for EdgeConfig {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let edge_name = &self.edge_name;
        let from = &self.from;
        let to = &self.to;
        let direction = &self.direction;

        tokens.extend(quote! {
            ::evenframe::schemasync::EdgeConfig {
                edge_name: #edge_name.to_string(),
                from: #from.to_string(),
                to: #to.to_string(),
                direction: #direction
            }
        });
    }
}

impl EdgeConfig {
    pub fn parse(field: &syn::Field) -> syn::Result<Option<EdgeConfig>> {
        let mut edge_name: Option<String> = None;
        let mut from: Option<String> = None;
        let mut to: Option<String> = None;
        let mut direction: Option<Direction> = None;

        // Iterate over all attributes of the field.
        for attr in &field.attrs {
            // Check if the attribute is an "edge" attribute.
            if attr.path().is_ident("edge") {
                attr.parse_nested_meta(|meta| {
                    // For "edge_name", ensure we only set it once.
                    if meta.path.is_ident("edge_name") {
                        let content;
                        parenthesized!(content in meta.input);
                        if edge_name.is_some() {
                            return Err(meta.error("duplicate edge_name attribute"));
                        }
                        edge_name = Some(content.parse::<syn::LitStr>()?.value());
                        return Ok(());
                    }
                    // For "from", ensure we only set it once.
                    if meta.path.is_ident("from") {
                        let content;
                        parenthesized!(content in meta.input);
                        if from.is_some() {
                            return Err(meta.error("duplicate from attribute"));
                        }
                        from = Some(content.parse::<syn::LitStr>()?.value());
                        return Ok(());
                    }
                    // For "to", ensure we only set it once.
                    if meta.path.is_ident("to") {
                        let content;
                        parenthesized!(content in meta.input);
                        if to.is_some() {
                            return Err(meta.error("duplicate to attribute"));
                        }
                        to = Some(content.parse::<syn::LitStr>()?.value());
                        return Ok(());
                    }

                    if meta.path.is_ident("direction") {
                        let content;
                        parenthesized!(content in meta.input);
                        if direction.is_some() {
                            return Err(meta.error("duplicate direction attribute"));
                        }
                        let lit: syn::LitStr = content.parse()?;
                        // Convert the string into a Direction using FromStr.
                        direction = Some(
                            lit.value()
                                .parse::<Direction>()
                                .map_err(|e| meta.error(e))?,
                        );
                        return Ok(());
                    }
                    // If an unexpected attribute is encountered, return an error.
                    Err(meta.error("unrecognized edge detail"))
                })?;
                // If any of the required attributes is missing, return an error indicating which one.
                let edge_name = edge_name
                    .ok_or_else(|| syn::Error::new(field.span(), "missing edge_name attribute"))?;
                let from =
                    from.ok_or_else(|| syn::Error::new(field.span(), "missing from attribute"))?;
                let to = to.ok_or_else(|| syn::Error::new(field.span(), "missing to attribute"))?;
                let direction = direction
                    .ok_or_else(|| syn::Error::new(field.span(), "missing direction attribute"))?;

                return Ok(Some(EdgeConfig {
                    edge_name,
                    from,
                    to,
                    direction,
                }));
            }
        }

        Ok(None)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Subquery {
    pub text: String,
}

impl Subquery {
    pub fn parse(field: &syn::Field) -> syn::Result<Option<Subquery>> {
        for attr in &field.attrs {
            if attr.path().is_ident("subquery") {
                // Parse the attribute content as a literal string.
                let lit: syn::LitStr = attr.parse_args()?;
                let text = lit.value();
                return Ok(Some(Subquery { text }));
            }
        }
        // If no subquery attribute is found, return None.
        Ok(None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Direction {
    From,
    To,
    Both,
}

impl ToTokens for Direction {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Direction::From => {
                tokens.extend(quote! { ::evenframe::schemasync::Direction::From })
            }
            Direction::To => {
                tokens.extend(quote! { ::evenframe::schemasync::Direction::To })
            }
            Direction::Both => {
                tokens.extend(quote! { ::evenframe::schemasync::Direction::To })
            }
        }
    }
}

impl FromStr for Direction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "from" => Ok(Direction::From),
            "to" => Ok(Direction::To),
            "both" => Ok(Direction::To),
            _ => Err(format!("Invalid direction: {}", s)),
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::From => write!(f, "From"),
            Direction::To => write!(f, "To"),
            Direction::Both => write!(f, "Both"),
        }
    }
}
