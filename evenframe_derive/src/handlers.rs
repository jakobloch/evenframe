use quote::quote;
use syn::{Ident, LitStr};

pub fn generate_handlers(ident: &Ident, subqueries: &[String], fetch_fields: &[String]) -> proc_macro2::TokenStream {
    let collection_name = helpers::case::to_snake_case(&ident.to_string());
    let collection_name_lit = LitStr::new(&collection_name, ident.span());
    
    // Create the edge subquery string fragments.
    let edge_query_part_read = if !subqueries.is_empty() {
        format!(", {}", subqueries.join(", "))
    } else {
        "".to_string()
    };
    let edge_query_part_fetch = if !subqueries.is_empty() {
        format!(", {}", subqueries.join(", "))
    } else {
        "".to_string()
    };

    // Create a FETCH clause for any fetch fields.
    let fetch_clause = if !fetch_fields.is_empty() {
        format!(" FETCH {}", fetch_fields.join(", "))
    } else {
        "".to_string()
    };
    
    let query_read1 = format!("SELECT *{} from ", edge_query_part_read);
    let query_read2 = format!("{};", fetch_clause);
    let query_fetch = format!(
        "SELECT *{} from {}{};",
        edge_query_part_fetch, collection_name, fetch_clause
    );
    let query_read1_lit = LitStr::new(&query_read1, ident.span());
    let query_read2_lit = LitStr::new(&query_read2, ident.span());
    let query_fetch_lit = LitStr::new(&query_fetch, ident.span());
    let create_fn = syn::Ident::new("create", ident.span());
    let update_fn = syn::Ident::new("update", ident.span());
    let delete_fn = syn::Ident::new("delete", ident.span());
    let read_fn = syn::Ident::new("read", ident.span());
    let fetch_fn = syn::Ident::new("fetch", ident.span());

    quote! {

        pub async fn #create_fn(
            State(state): State<helpers::app_state::AppState>,
            jar: axum_extra::extract::PrivateCookieJar,
            axum_extra::TypedHeader(host): axum_extra::TypedHeader<headers::Host>,
            Json(payload): Json<#ident>,
        ) -> Result<(StatusCode, Json<#ident>), Error> {
            dotenv::dotenv().ok(); // Load .env file
            let query = ::helpers::evenframe::schemasync::generate_query(::helpers::evenframe::schemasync::QueryType::Create, &payload.get_table_config().unwrap(), &payload, None);
            let _ = ureq::post("http://localhost:8000/sql")
            .header("Authorization", &format!("Bearer {}", &jar.get("auth_token").unwrap().to_string()[11..]))
                .header("Accept", "application/json")
                .header(
                    "Surreal-NS",
                    std::env::var("SURREAL_NAMESPACE").expect("Surreal namespace not set"),
                )
                .header(
                    "Surreal-DB",
                    std::env::var("SURREAL_DATABASE").expect("Surreal database not set"),
                )
            .send(query)
            .unwrap()
            .body_mut()
            .read_json::<Vec<helpers::database::Response<serde_json::Value>>>().unwrap();

            let item = #ident::#read_fn(State(state), jar, axum_extra::TypedHeader(host), Json(payload.id))
                .await?
                .1;
            Ok((StatusCode::OK, item))
        }


        pub async fn #update_fn(
            State(state): State<helpers::app_state::AppState>,
            jar: axum_extra::extract::PrivateCookieJar,
            axum_extra::TypedHeader(host): axum_extra::TypedHeader<headers::Host>,
            Json(payload): Json<#ident>,
        ) -> Result<(StatusCode, Json<#ident>), Error> {
            dotenv::dotenv().ok(); // Load .env file
            let query = ::helpers::evenframe::schemasync::generate_query(::helpers::evenframe::schemasync::QueryType::Update, &payload.get_table_config().unwrap(), &payload, None);
            let subdomain = &helpers::subdomain::Subdomain::get_subdomain(&host);
            let db_name = &state.clients.get(subdomain).unwrap().0;

            let _ = ureq::post("http://localhost:8000/sql")
            .header("Authorization", &format!("Bearer {}", &jar.get("auth_token").unwrap().to_string()[11..]))
                .header("Accept", "application/json")
                .header(
                    "Surreal-NS",
                    std::env::var("SURREAL_NAMESPACE").expect("Surreal namespace not set"),
                )
                .header(
                    "Surreal-DB",
                    db_name,
                )
            .send(query)
            .unwrap()
            .body_mut()
            .read_json::<Vec<helpers::database::Response<serde_json::Value>>>().unwrap();

            let item = #ident::#read_fn(State(state), jar, axum_extra::TypedHeader(host), Json(payload.id))
                .await?
                .1;

            Ok((StatusCode::OK, item))
        }


        pub async fn #delete_fn(
            State(state): State<helpers::app_state::AppState>,
            axum_extra::TypedHeader(host): axum_extra::TypedHeader<headers::Host>,
            Json(payload): Json<surrealdb::RecordId>,
        ) -> Result<(StatusCode, Json<#ident>), Error> {
            let subdomain = &helpers::subdomain::Subdomain::get_subdomain(&host);
            let db = &state.clients.get(subdomain).unwrap().1;


            let item: #ident = db
                .delete((#collection_name_lit, payload.to_string()))
                .await?
                .ok_or(Error::Db)?;
            Ok((StatusCode::OK, Json(item)))
        }


        pub async fn #read_fn(
            State(state): State<helpers::app_state::AppState>,
            jar: axum_extra::extract::PrivateCookieJar,
            axum_extra::TypedHeader(host): axum_extra::TypedHeader<headers::Host>,
            Json(payload): Json<::helpers::evenframe::wrappers::EvenframeRecordId>,
        ) -> Result<(StatusCode, Json<#ident>), Error> {
            dotenv::dotenv().ok(); // Load .env file
            let query = format!("{}{}{}", #query_read1_lit, payload.to_string().replace("⟩", "").replace("⟨", ""), #query_read2_lit);
            let subdomain = &helpers::subdomain::Subdomain::get_subdomain(&host);
            let db = &state.clients.get(subdomain).unwrap().1;
            let db_name = &state.clients.get(subdomain).unwrap().0;



            db.set("payload", payload.to_string().replace("⟩", "").replace("⟨", "")).await?;
            let response = ureq::post("http://localhost:8000/sql")
            .header("Authorization", &format!("Bearer {}", &jar.get("auth_token").unwrap().to_string()[11..]))
                .header("Accept", "application/json")
                .header(
                    "Surreal-NS",
                    std::env::var("SURREAL_NAMESPACE").expect("Surreal namespace not set"),
                )
                .header(
                    "Surreal-DB",
                    db_name,
                )
            .send(query)
            .unwrap()
            .body_mut()
            .read_json::<Vec<helpers::database::Response<#ident>>>().unwrap();
            db.unset("payload").await?;


            helpers::utils::log(&format!("{:?}", response[0].result.clone()), "logs/debug.log", true);

            Ok((StatusCode::OK, Json(response[0].result[0].clone())))
        }


        pub async fn #fetch_fn(
            State(state): State<helpers::app_state::AppState>,
            jar: axum_extra::extract::PrivateCookieJar,
            axum_extra::TypedHeader(host): axum_extra::TypedHeader<headers::Host>,
        ) -> Result<(StatusCode, Json<Vec<#ident>>), Error> {
            dotenv::dotenv().ok(); // Load .env file
            let query = #query_fetch_lit;
            let subdomain = &helpers::subdomain::Subdomain::get_subdomain(&host);
            let db_name = &state.clients.get(subdomain).unwrap().0;

            let response = ureq::post("http://localhost:8000/sql")
                    .header("Authorization", &format!("Bearer {}", &jar.get("auth_token").unwrap().to_string()[11..]))
                    .header("Accept", "application/json")
                    .header(
                        "Surreal-NS",
                        std::env::var("SURREAL_NAMESPACE").expect("Surreal namespace not set"),
                    )
                    .header(
                        "Surreal-DB",
                        db_name,
                    )
                    .send(format!("{}", query))
                    .unwrap()
                    .body_mut()
                    .read_json::<Vec<helpers::database::Response<#ident>>>().unwrap();


            helpers::utils::log(&format!("{:?}", response[0].result.clone()), "logs/debug.log", true);

            Ok((StatusCode::OK, Json(response[0].result.clone())))
        }
    }
}