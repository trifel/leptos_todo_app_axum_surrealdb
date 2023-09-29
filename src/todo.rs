use crate::error_template::ErrorTemplate;
use cfg_if::cfg_if;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Todo {
    id: Option<String>,
    title: String,
    completed: bool,
}

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use surrealdb::{Surreal, engine::remote::ws::Client};
        use surrealdb::sql::{Thing, Id};

        const TODO_RESOURCE: &str = "todo";

        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub struct TodoRecord {
            id: Option<Thing>,
            title: String,
            completed: bool,
        }

        pub async fn db() -> Result<Surreal<Client>, ServerFnError> {
            use_context::<Surreal<Client>>()
            .ok_or_else(|| ServerFnError::ServerError("Database connection missing.".into()))
        }
    }
}

#[server(GetTodos, "/api")]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    // this is just an example of how to access server context injected in the handlers
    // http::Request doesn't implement Clone, so more work will be needed to do use_context() on this
    let req_parts = use_context::<leptos_axum::RequestParts>();

    if let Some(req_parts) = req_parts {
        println!("Uri = {:?}", req_parts.uri);
    }
    let db = db().await?;
    let todos = db.select(TODO_RESOURCE).await?;
    Ok(todos
        .into_iter()
        .map(|todo: TodoRecord| Todo {
            id: todo.id.map(|thing| thing.id.to_string()),
            title: todo.title,
            completed: todo.completed,
        })
        .collect())
}

#[server(AddTodo, "/api")]
pub async fn add_todo(title: String) -> Result<(), ServerFnError> {
    let db = db().await?;

    // fake API delay
    std::thread::sleep(std::time::Duration::from_millis(1250));

    let new_todo: Result<Vec<TodoRecord>, surrealdb::Error> = db
        .create(TODO_RESOURCE)
        .content(TodoRecord {
            id: None,
            title,
            completed: false,
        })
        .await;

    match new_todo {
        Ok(_) => Ok(()),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

// The struct name and path prefix arguments are optional.
#[server]
pub async fn delete_todo(id: String) -> Result<(), ServerFnError> {
    let db = db().await?;
    let res: Result<Option<TodoRecord>, surrealdb::Error> =
        db.delete((TODO_RESOURCE, Id::from(id))).await;

    match res {
        Ok(_) => Ok(()),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

#[component]
pub fn TodoApp() -> impl IntoView {
    provide_meta_context();
    view! {
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Stylesheet id="leptos" href="/pkg/leptos_todo_app_axum_surrealdb.css"/>
        <Router>
            <header>
                <h1>"My Tasks"</h1>
            </header>
            <main>
                <Routes>
                    <Route path="" view=Todos/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn Todos() -> impl IntoView {
    let add_todo = create_server_multi_action::<AddTodo>();
    let delete_todo = create_server_action::<DeleteTodo>();
    let submissions = add_todo.submissions();

    // list of todos is loaded from the server in reaction to changes
    let todos = create_resource(
        move || (add_todo.version().get(), delete_todo.version().get()),
        move |_| get_todos(),
    );

    view! {
        <div>
            <MultiActionForm action=add_todo>
                <label>
                    "Add a Todo"
                    <input type="text" name="title"/>
                </label>
                <input type="submit" value="Add"/>
            </MultiActionForm>
            <Transition fallback=move || view! {<p>"Loading..."</p> }>
                <ErrorBoundary fallback=|errors| view!{<ErrorTemplate errors=errors/>}>
                    {move || {
                        let existing_todos = {
                            move || {
                                todos.get()
                                    .map(move |todos| match todos {
                                        Err(e) => {
                                            view! { <pre class="error">"Server Error: " {e.to_string()}</pre>}.into_view()
                                        }
                                        Ok(todos) => {
                                            if todos.is_empty() {
                                                view! { <p>"No tasks were found."</p> }.into_view()
                                            } else {
                                                todos
                                                    .into_iter()
                                                    .map(move |todo| {
                                                        view! {

                                                            <li>
                                                                {todo.title}
                                                                <ActionForm action=delete_todo>
                                                                    <input type="hidden" name="id" value={todo.id.unwrap_or("".into())}/>
                                                                    <input type="submit" value="X"/>
                                                                </ActionForm>
                                                            </li>
                                                        }
                                                    })
                                                    .collect_view()
                                            }
                                        }
                                    })
                                    .unwrap_or_default()
                            }
                        };

                        let pending_todos = move || {
                            submissions
                            .get()
                            .into_iter()
                            .filter(|submission| submission.pending().get())
                            .map(|submission| {
                                view! {

                                    <li class="pending">{move || submission.input.get().map(|data| data.title) }</li>
                                }
                            })
                            .collect_view()
                        };

                        view! {

                            <ul>
                                {existing_todos}
                                {pending_todos}
                            </ul>
                        }
                    }
                }
                </ErrorBoundary>
            </Transition>
        </div>
    }
}
