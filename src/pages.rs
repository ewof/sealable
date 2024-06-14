use askama_axum::Template;
use axum::{
    extract::{Path, State, Query},
    response::{Html, Json, IntoResponse},
    routing::{get, post, get_service},
    Router,
};

use tower_http::services::ServeDir;
use pastemd::{database::Database, model::Paste};
use sauropod::markdown::parse_markdown as shared_parse_markdown;
use serde::{Serialize, Deserialize};

pub fn routes(database: Database) -> Router {
    Router::new()
        .route("/:url/edit/config", get(config_editor_request))
        .route("/:url/edit", get(editor_request))
        .route("/:url", get(view_paste_request))
        .route("/api/render", post(render_markdown))
        // serve static dir
        .nest_service("/static", get_service(ServeDir::new("./static")))
        // ...
        .with_state(database)
}

#[derive(Template)]
#[template(path = "homepage.html")]
struct HomepageTemplate {}

pub async fn homepage() -> impl IntoResponse {
    Html(HomepageTemplate {}.render().unwrap())
}

#[derive(Template)]
#[template(path = "paste_view.html")]
struct PasteViewTemplate {
    paste: Paste,
    rendered: String,
    title: String,
    views: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PasteViewQuery {
    #[serde(default)]
    view_password: String,
}

#[derive(Template)]
#[template(path = "paste_password.html")]
struct PastePasswordTemplate {
    paste: Paste,
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorViewTemplate {
    error: String,
}

pub async fn view_paste_request(
    Path(url): Path<String>,
    State(database): State<Database>,
    Query(query_params): Query<PasteViewQuery>,
) -> impl IntoResponse {
    match database.get_paste_by_url(url).await {
        Ok(p) => {
            // push view
            if let Err(e) = database.incr_views_by_url(p.url.clone()).await {
                return Html(
                    ErrorViewTemplate {
                        error: e.to_string(),
                    }
                    .render()
                    .unwrap(),
                );
            }

            // check for view password
            if database.options.view_password == true {
                match query_params.view_password.is_empty() {
                    false => {
                        if !p.metadata.view_password.is_empty()
                            && (query_params.view_password != p.metadata.view_password)
                        {
                            return Html(PastePasswordTemplate { paste: p }.render().unwrap());
                        }
                    }
                    true => {
                        if !p.metadata.view_password.is_empty() {
                            return Html(PastePasswordTemplate { paste: p }.render().unwrap());
                        }
                    }
                }
            }

            // ...
            let rendered = shared_parse_markdown(p.content.clone(), Vec::new());
            Html(
                PasteViewTemplate {
                    paste: p.clone(),
                    rendered,
                    title: match p.metadata.title.is_empty() {
                        true => p.url.clone(),
                        false => p.metadata.title,
                    },
                    views: database.get_views_by_url(p.url).await,
                }
                .render()
                .unwrap(),
            )
        }
        Err(e) => Html(
            ErrorViewTemplate {
                error: e.to_string(),
            }
            .render()
            .unwrap(),
        ),
    }
}

#[derive(Template)]
#[template(path = "paste_editor.html")]
struct EditorTemplate {
    paste: Paste,
}

pub async fn editor_request(
    Path(url): Path<String>,
    State(database): State<Database>,
    Query(query_params): Query<PasteViewQuery>,
) -> impl IntoResponse {
    match database.get_paste_by_url(url).await {
        Ok(p) => {
            // check for view password
            if database.options.view_password == true {
                match query_params.view_password.is_empty() {
                    false => {
                        if !p.metadata.view_password.is_empty()
                            && (query_params.view_password != p.metadata.view_password)
                        {
                            return Html(PastePasswordTemplate { paste: p }.render().unwrap());
                        }
                    }
                    true => {
                        if !p.metadata.view_password.is_empty() {
                            return Html(PastePasswordTemplate { paste: p }.render().unwrap());
                        }
                    }
                }
            }

            // ...
            Html(EditorTemplate { paste: p }.render().unwrap())
        }
        Err(e) => Html(
            ErrorViewTemplate {
                error: e.to_string(),
            }
            .render()
            .unwrap(),
        ),
    }
}

#[derive(Template)]
#[template(path = "paste_metadata.html")]
struct ConfigEditorTemplate {
    paste: Paste,
    paste_metadata: String,
}

pub async fn config_editor_request(
    Path(url): Path<String>,
    State(database): State<Database>,
    Query(query_params): Query<PasteViewQuery>,
) -> impl IntoResponse {
    match database.get_paste_by_url(url).await {
        Ok(p) => {
            // check for view password
            if database.options.view_password == true {
                match query_params.view_password.is_empty() {
                    false => {
                        if !p.metadata.view_password.is_empty()
                            && (query_params.view_password != p.metadata.view_password)
                        {
                            return Html(PastePasswordTemplate { paste: p }.render().unwrap());
                        }
                    }
                    true => {
                        if !p.metadata.view_password.is_empty() {
                            return Html(PastePasswordTemplate { paste: p }.render().unwrap());
                        }
                    }
                }
            }

            // ...
            Html(
                ConfigEditorTemplate {
                    paste: p.clone(),
                    paste_metadata: match serde_json::to_string(&p.metadata) {
                        Ok(m) => m,
                        Err(_) => {
                            return Html(
                                ErrorViewTemplate {
                                    error: pastemd::model::PasteError::Other.to_string(),
                                }
                                .render()
                                .unwrap(),
                            )
                        }
                    },
                }
                .render()
                .unwrap(),
            )
        }
        Err(e) => Html(
            ErrorViewTemplate {
                error: e.to_string(),
            }
            .render()
            .unwrap(),
        ),
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct RenderMarkdown {
    pub content: String,
}

/// Render markdown body
async fn render_markdown(Json(req): Json<RenderMarkdown>) -> Result<String, ()> {
    Ok(shared_parse_markdown(req.content.clone(), Vec::new()))
}
