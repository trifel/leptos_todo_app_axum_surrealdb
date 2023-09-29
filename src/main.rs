use cfg_if::cfg_if;
// boilerplate to run in different modes
cfg_if! {
    if #[cfg(feature = "ssr")] {
    use leptos::*;
    use axum::{
        routing::{post, get},
        extract::{State, Path, RawQuery, FromRef},
        http::{Request, HeaderMap},
        response::{IntoResponse, Response},
        Router,
    };
    use axum::body::Body as AxumBody;
    use crate::todo::*;
    use leptos_todo_app_axum_surrealdb::*;
    use crate::fallback::file_and_error_handler;
    use leptos_axum::{generate_route_list, LeptosRoutes, handle_server_fns_with_context};
    use surrealdb::engine::remote::ws::Ws;
    use surrealdb::opt::auth::Root;
    use surrealdb::{Surreal, engine::remote::ws::Client};
    use std::env;

    #[derive(FromRef, Debug, Clone)]
    pub struct AppState{
        pub leptos_options: LeptosOptions,
        pub db: Surreal<Client>,
    }

    async fn server_fn_handler(State(app_state): State<AppState>, path: Path<String>, headers: HeaderMap, raw_query: RawQuery,
    request: Request<AxumBody>) -> impl IntoResponse {
        handle_server_fns_with_context(path, headers, raw_query, move || {
            provide_context(app_state.db.clone());
        }, request).await
    }

    async fn leptos_routes_handler(State(app_state): State<AppState>, req: Request<AxumBody>) -> Response{
            let handler = leptos_axum::render_app_to_stream_with_context(app_state.leptos_options.clone(),
            move || {
                provide_context(app_state.db.clone());
            },
            || view! { <TodoApp/> }
        );
        handler(req).await.into_response()
    }

    #[tokio::main]
    async fn main() {
        simple_logger::init_with_level(log::Level::Error).expect("couldn't initialize logging");

        // Connect to the surrealDB server
        let surrealdb_server = env::var("SURREALDB_SERVER").unwrap_or("127.0.0.1".to_string());
        let surrealdb_port = env::var("SURREALDB_PORT").unwrap_or("8000".to_string());
        let surrealdb_username = env::var("SURREALDB_USERNAME").unwrap_or("root".to_string());
        let surrealdb_password = env::var("SURREALDB_PASSWORD").unwrap_or("root".to_string());
        let surrealdb_ns = env::var("SURREALDB_NS").unwrap_or("leptos_examples".to_string());
        let surrealdb_db = env::var("SURREALDB_DB").unwrap_or("todos".to_string());

        let db = Surreal::new::<Ws>(format!("{}:{}", surrealdb_server, surrealdb_port)).await.expect("couldn't connect to surrealDB server");
        db.signin(Root {
            username: &surrealdb_username,
            password: &surrealdb_password,
        })
        .await.expect("couldn't signin to surrealDB server");
        db.use_ns(surrealdb_ns).use_db(surrealdb_db).await.expect("couldn't find db on surrealDB server");

        // Setting this to None means we'll be using cargo-leptos and its env vars
        let conf = get_configuration(None).await.unwrap();
        let leptos_options = conf.leptos_options;
        let addr = leptos_options.site_addr;
        let routes = generate_route_list(|| view! { <TodoApp/> });

        let app_state = AppState{
            leptos_options,
            db: db.clone(),
        };

        // build our application with a route
        let app = Router::new()
        .route("/api/*fn_name", post(server_fn_handler))
        .leptos_routes_with_handler(routes, get(leptos_routes_handler) )
        .fallback(file_and_error_handler)
        .with_state(app_state);

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        logging::log!("listening on http://{}", &addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
        }
    }

    // client-only stuff for Trunk
    else {
        pub fn main() {
            // This example cannot be built as a trunk standalone CSR-only app.
            // Only the server may directly connect to the database.
        }
    }
}
