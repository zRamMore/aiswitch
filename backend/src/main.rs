#[macro_use]
extern crate rocket;

use config::{AppConfig, Preset, ProviderConfig};
use log::warn;
use reqwest::Client;
use rocket::fs::NamedFile;
use rocket::response::stream::TextStream;
use rocket::serde::Deserialize;
use rocket::tokio::sync::{mpsc, Mutex};
use rocket::{get, post, put, routes, serde::json::Json, State};
use rocket_cors::AllowedOrigins;
use rusqlite::{params, Connection};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::Instant;

mod config;

static DB_CONNECTION: LazyLock<Arc<Mutex<Option<Connection>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

type SharedConfig = Arc<Mutex<AppConfig>>;

/// Tokenizes a prompt using the selected provider, if it supports tokenization
async fn tokenize_external(
    provider: &ProviderConfig,
    model: &str,
    prompt: &str,
) -> Result<Vec<u64>, rocket::http::Status> {
    #[derive(Deserialize)]
    #[serde(crate = "rocket::serde")]
    struct TokenizeResponse {
        tokens: Vec<u64>,
    }

    let api_url = format!("{}/tokenize", provider.api_url);
    let client = Client::new();
    let mut body = serde_json::json!({
        "model": model,
        "prompt": prompt
    });
    if let Some(preset) = &provider.preset {
        if let Some(preset) = provider.presets.iter().find(|p| &p.id == preset) {
            for prop in preset.overrides.iter() {
                body[prop.0] = prop.1.clone();
            }
        }
    }

    let res = client
        .post(&api_url)
        .header("Authorization", format!("Bearer {}", provider.api_key))
        .json(&body)
        .send()
        .await;

    match res {
        Ok(response) => match response.text().await {
            Ok(text) => {
                let response: Result<TokenizeResponse, _> = serde_json::from_str(&text);
                match response {
                    Ok(response) => Ok(response.tokens),
                    Err(_) => Err(rocket::http::Status::ServiceUnavailable),
                }
            }
            Err(_) => Err(rocket::http::Status::ServiceUnavailable),
        },
        Err(_) => Err(rocket::http::Status::ServiceUnavailable),
    }
}

async fn tokenize(provider: &ProviderConfig, model: &str, prompt: &str) -> Option<Vec<u64>> {
    if let Ok(tokenized) = tokenize_external(provider, model, prompt).await {
        return Some(tokenized);
    }
    None
}

enum CompletionPrompt {
    String(String),
    Array(Vec<String>),
    Tokens(Vec<u64>),
}

#[post("/api/v1/completions", data = "<body>")]
async fn proxy_completions(
    body: Json<HashMap<String, serde_json::Value>>,
    config: &State<SharedConfig>,
) -> Result<Result<String, TextStream![String]>, rocket::http::Status> {
    let config = config.lock().await;
    let provider_id = match &config.provider {
        Some(provider_id) => provider_id.clone(),
        None => return Err(rocket::http::Status::ServiceUnavailable),
    };

    let selected_provider = match config.providers.iter().find(|p| p.id == provider_id) {
        Some(provider) => provider.clone(),
        None => return Err(rocket::http::Status::ServiceUnavailable),
    };

    let api_url = format!("{}/completions", selected_provider.api_url);
    let client = Client::new();

    let mut modified_body = body.into_inner();
    if let Some(preset) = &selected_provider.preset {
        if let Some(preset) = selected_provider.presets.iter().find(|p| &p.id == preset) {
            for prop in preset.overrides.iter() {
                modified_body.insert(prop.0.to_owned(), prop.1.to_owned());
            }
        }
    }

    let prompt = modified_body
        .get("prompt")
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                Some(CompletionPrompt::String(s.to_owned()))
            } else if let Some(a) = v.as_array() {
                let tokens = a.get(0).is_some_and(|v| v.is_u64());
                if tokens {
                    let tokens = a.iter().filter_map(|v| v.as_u64()).collect::<Vec<u64>>();
                    Some(CompletionPrompt::Tokens(tokens))
                } else {
                    let strings = a
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_owned())
                        .collect::<Vec<String>>();
                    Some(CompletionPrompt::Array(strings))
                }
            } else {
                None
            }
        })
        .unwrap_or(CompletionPrompt::String("".to_owned()));

    let model = modified_body
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-3.5-turbo-instruct")
        .to_owned();

    let stream = modified_body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if stream {
        // Force include_usage
        modified_body
            .entry("stream_options".to_string())
            .or_insert(serde_json::json!({}))
            .as_object_mut()
            .map(|o| {
                o.insert("include_usage".to_string(), serde_json::json!(true));
            });
    }

    let id = {
        let db_lock = DB_CONNECTION.lock().await;
        let db = db_lock.as_ref().unwrap();
        db
        .execute(
            "INSERT INTO requests (provider_id, chat, request, request_time, model) VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP, ?4)",
            params![
                provider_id,
                false,
                serde_json::to_string(&modified_body).unwrap(),
                model.clone()
            ],
        )
        .unwrap();
        // As we have the connection locked, it is guranteed that this is the id of the request we just inserted
        db.last_insert_rowid()
    };

    let time = Instant::now();

    if stream {
        let (tx, mut rx) = mpsc::channel(32);
        rocket::tokio::spawn(async move {
            let res = client
                .post(&api_url)
                .header(
                    "Authorization",
                    format!("Bearer {}", selected_provider.api_key),
                )
                .json(&modified_body)
                .send()
                .await;

            let mut log = Vec::new();
            let mut prompt_tokens = match prompt {
                CompletionPrompt::Tokens(ref tokens) => Some(tokens.len() as u64),
                _ => None,
            };
            let mut completion_tokens = None;
            let mut speed = None;
            let mut text_to_count = String::new();

            match res {
                Ok(mut response) => {
                    while let Some(chunk) = response.chunk().await.unwrap() {
                        let chunk = String::from_utf8_lossy(&chunk).to_string();
                        if let Some((_, body)) = chunk.split_once(':') {
                            let chunk: Result<serde_json::Map<String, serde_json::Value>, _> =
                                serde_json::from_str(body);
                            if let Ok(chunk) = chunk {
                                if let Some(usage) = chunk.get("usage").and_then(|u| u.as_object())
                                {
                                    prompt_tokens = usage
                                        .get("prompt_tokens")
                                        .and_then(|t| t.as_u64())
                                        .or(prompt_tokens);
                                    completion_tokens = usage
                                        .get("completion_tokens")
                                        .and_then(|t| t.as_u64())
                                        .or(completion_tokens);
                                } else if let Some(choices) =
                                    chunk.get("choices").and_then(|c| c.as_array())
                                {
                                    for choice in choices {
                                        if let Some(text) =
                                            choice.get("text").and_then(|c| c.as_str())
                                        {
                                            text_to_count.push_str(text);
                                        }
                                    }
                                }
                                log.push(chunk);
                            }
                        }
                        let _ = tx.send(chunk).await;
                    }

                    if prompt_tokens.is_none() {
                        let prompt = match prompt {
                            CompletionPrompt::String(s) => s,
                            CompletionPrompt::Array(a) => a.join("\n"),
                            CompletionPrompt::Tokens(_) => unreachable!(),
                        };
                        if let Some(tokens) = tokenize(&selected_provider, &model, &prompt).await {
                            prompt_tokens = Some(tokens.len() as u64);
                        }
                    }

                    if completion_tokens.is_none() {
                        if let Some(tokens) =
                            tokenize(&selected_provider, &model, &text_to_count).await
                        {
                            completion_tokens = Some(tokens.len() as u64);
                        }
                    }

                    if let Some(completion_tokens) = completion_tokens {
                        let elapsed = time.elapsed();
                        let elapsed = elapsed.as_secs_f64();
                        speed = Some((completion_tokens as f64 / elapsed) as i64);
                    }

                    DB_CONNECTION.lock().await.as_ref().unwrap()
                        .execute(
                            "UPDATE requests SET response = ?2, response_time = CURRENT_TIMESTAMP, prompt_tokens = ?3, completion_tokens = ?4, speed = ?5 WHERE id = ?1",
                            params![id.to_string(), serde_json::to_string(&log).unwrap(), prompt_tokens, completion_tokens, speed],
                        )
                        .unwrap();
                }
                Err(_) => {
                    let _ = tx.send("Error streaming response".to_string()).await;
                }
            }
        });

        Ok(Err(rocket::response::stream::TextStream! {
            while let Some(chunk) = rx.recv().await {
                yield chunk;
            }
        }))
    } else {
        let res = client
            .post(&api_url)
            .header(
                "Authorization",
                format!("Bearer {}", selected_provider.api_key),
            )
            .json(&modified_body)
            .send()
            .await;

        match res {
            Ok(response) => match response.text().await {
                Ok(text) => {
                    let mut prompt_tokens = match prompt {
                        CompletionPrompt::Tokens(ref tokens) => Some(tokens.len() as u64),
                        _ => None,
                    };
                    let mut completion_tokens = None;
                    let mut speed = None;
                    let mut text_to_count = String::new();
                    let json: Result<serde_json::Map<String, serde_json::Value>, _> =
                        serde_json::from_str(&text);
                    if let Ok(json) = json {
                        if let Some(usage) = json.get("usage").and_then(|u| u.as_object()) {
                            prompt_tokens = usage
                                .get("prompt_tokens")
                                .and_then(|t| t.as_u64())
                                .or(prompt_tokens);
                            completion_tokens = usage
                                .get("completion_tokens")
                                .and_then(|t| t.as_u64())
                                .or(completion_tokens);
                        } else if let Some(choices) = json.get("choices").and_then(|c| c.as_array())
                        {
                            for choice in choices {
                                if let Some(text) = choice.get("text").and_then(|c| c.as_str()) {
                                    text_to_count.push_str(text);
                                }
                            }
                        }
                    }

                    if prompt_tokens.is_none() {
                        let prompt = match prompt {
                            CompletionPrompt::String(s) => s,
                            CompletionPrompt::Array(a) => a.join("\n"),
                            CompletionPrompt::Tokens(_) => unreachable!(),
                        };
                        if let Some(tokens) = tokenize(&selected_provider, &model, &prompt).await {
                            prompt_tokens = Some(tokens.len() as u64);
                        }
                    }
                    if completion_tokens.is_none() {
                        if let Some(tokens) =
                            tokenize(&selected_provider, &model, &text_to_count).await
                        {
                            completion_tokens = Some(tokens.len() as u64);
                        }
                    }

                    if let Some(completion_tokens) = completion_tokens {
                        let elapsed = time.elapsed();
                        let elapsed = elapsed.as_secs_f64();
                        speed = Some((completion_tokens as f64 / elapsed) as i64);
                    }

                    DB_CONNECTION.lock().await.as_ref().unwrap()
                        .execute(
                            "UPDATE requests SET response = ?1, response_time = CURRENT_TIMESTAMP, prompt_tokens = ?3, completion_tokens = ?4, speed = ?5 WHERE id = ?2",
                            params![text.clone(), id.to_string(), prompt_tokens, completion_tokens, speed],
                        )
                        .unwrap();
                    Ok(Ok(text))
                }
                Err(_) => Err(rocket::http::Status::ServiceUnavailable),
            },
            Err(_) => Err(rocket::http::Status::ServiceUnavailable),
        }
    }
}

#[post("/api/v1/chat/completions", data = "<body>")]
async fn proxy_chat_completions(
    body: Json<HashMap<String, serde_json::Value>>,
    config: &State<SharedConfig>,
) -> Result<Result<String, TextStream![String]>, rocket::http::Status> {
    let config = config.lock().await;
    let provider_id = match &config.provider {
        Some(provider_id) => provider_id.clone(),
        None => return Err(rocket::http::Status::ServiceUnavailable),
    };

    let selected_provider = match config.providers.iter().find(|p| p.id == provider_id) {
        Some(provider) => provider.clone(),
        None => return Err(rocket::http::Status::ServiceUnavailable),
    };

    let api_url = format!("{}/chat/completions", selected_provider.api_url);
    let client = Client::new();

    let mut modified_body = body.into_inner();
    if let Some(preset) = &selected_provider.preset {
        if let Some(preset) = selected_provider.presets.iter().find(|p| &p.id == preset) {
            for prop in preset.overrides.iter() {
                modified_body.insert(prop.0.to_owned(), prop.1.to_owned());
            }
        }
    }

    let model = modified_body
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-3.5-turbo")
        .to_owned();

    let messages = modified_body
        .get("messages")
        .and_then(|v| {
            v.as_array().and_then(|m| {
                m.into_iter()
                    .map(|v| {
                        v.as_object().and_then(|st| {
                            st.get("content")
                                .and_then(|c| c.as_str().map(|s| s.to_owned()))
                        })
                    })
                    .collect()
            })
        })
        .unwrap_or(Vec::new());

    let stream = modified_body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if stream {
        // Force include_usage
        modified_body
            .entry("stream_options".to_string())
            .or_insert(serde_json::json!({}))
            .as_object_mut()
            .map(|o| {
                o.insert("include_usage".to_string(), serde_json::json!(true));
            });
    }

    let id = {
        let db_lock = DB_CONNECTION.lock().await;
        let db = db_lock.as_ref().unwrap();
        db
        .execute(
            "INSERT INTO requests (provider_id, chat, request, request_time, model) VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP, ?4)",
            params![
                provider_id,
                true,
                serde_json::to_string(&modified_body).unwrap(),
                model.clone()
            ],
        )
        .unwrap();
        // As we have the connection locked, it is guranteed that this is the id of the request we just inserted
        db.last_insert_rowid()
    };

    let time = Instant::now();

    if stream {
        let (tx, mut rx) = mpsc::channel(32);
        rocket::tokio::spawn(async move {
            let res = client
                .post(&api_url)
                .header(
                    "Authorization",
                    format!("Bearer {}", selected_provider.api_key),
                )
                .json(&modified_body)
                .send()
                .await;

            let mut log = Vec::new();
            let mut prompt_tokens = None;
            let mut completion_tokens = None;
            let mut speed = None;
            let mut text_to_count = String::new();

            match res {
                Ok(mut response) => {
                    while let Some(chunk) = response.chunk().await.unwrap() {
                        let chunk = String::from_utf8_lossy(&chunk).to_string();
                        if let Some((_, body)) = chunk.split_once(':') {
                            let chunk: Result<serde_json::Map<String, serde_json::Value>, _> =
                                serde_json::from_str(body);
                            if let Ok(chunk) = chunk {
                                if let Some(usage) = chunk.get("usage").and_then(|u| u.as_object())
                                {
                                    prompt_tokens = usage
                                        .get("prompt_tokens")
                                        .and_then(|t| t.as_u64())
                                        .or(prompt_tokens);
                                    completion_tokens = usage
                                        .get("completion_tokens")
                                        .and_then(|t| t.as_u64())
                                        .or(completion_tokens);
                                } else if let Some(choices) =
                                    chunk.get("choices").and_then(|c| c.as_array())
                                {
                                    for choice in choices {
                                        if let Some(text) = choice.get("delta").and_then(|d| {
                                            d.as_object().and_then(|o| {
                                                o.get("content").and_then(|t| t.as_str())
                                            })
                                        }) {
                                            text_to_count.push_str(text);
                                        }
                                    }
                                }
                                log.push(chunk);
                            }
                        }
                        let _ = tx.send(chunk).await;
                    }

                    if prompt_tokens.is_none() {
                        if let Some(tokens) =
                            tokenize(&selected_provider, &model, &messages.join("\n")).await
                        {
                            prompt_tokens = Some(tokens.len() as u64);
                        }
                    }

                    if completion_tokens.is_none() {
                        if let Some(tokens) =
                            tokenize(&selected_provider, &model, &text_to_count).await
                        {
                            completion_tokens = Some(tokens.len() as u64);
                        }
                    }

                    if let Some(completion_tokens) = completion_tokens {
                        let elapsed = time.elapsed();
                        let elapsed = elapsed.as_secs_f64();
                        speed = Some((completion_tokens as f64 / elapsed) as i64);
                    }

                    DB_CONNECTION.lock().await.as_ref().unwrap()
                        .execute(
                            "UPDATE requests SET response = ?2, response_time = CURRENT_TIMESTAMP, prompt_tokens = ?3, completion_tokens = ?4, speed = ?5 WHERE id = ?1",
                            params![id.to_string(), serde_json::to_string(&log).unwrap(), prompt_tokens, completion_tokens, speed],
                        )
                        .unwrap();
                }
                Err(_) => {
                    let _ = tx.send("Error streaming response".to_string()).await;
                }
            }
        });

        Ok(Err(rocket::response::stream::TextStream! {
            while let Some(chunk) = rx.recv().await {
                yield chunk;
            }
        }))
    } else {
        let res = client
            .post(&api_url)
            .header(
                "Authorization",
                format!("Bearer {}", selected_provider.api_key),
            )
            .json(&modified_body)
            .send()
            .await;

        match res {
            Ok(response) => match response.text().await {
                Ok(text) => {
                    let mut prompt_tokens = None;
                    let mut completion_tokens = None;
                    let mut speed = None;
                    let mut text_to_count = String::new();
                    let json: Result<serde_json::Map<String, serde_json::Value>, _> =
                        serde_json::from_str(&text);
                    if let Ok(json) = json {
                        if let Some(usage) = json.get("usage").and_then(|u| u.as_object()) {
                            prompt_tokens = usage
                                .get("prompt_tokens")
                                .and_then(|t| t.as_u64())
                                .or(prompt_tokens);
                            completion_tokens = usage
                                .get("completion_tokens")
                                .and_then(|t| t.as_u64())
                                .or(completion_tokens);
                        } else if let Some(choices) = json.get("choices").and_then(|c| c.as_array())
                        {
                            for choice in choices {
                                if let Some(text) = choice.get("message").and_then(|d| {
                                    d.as_object()
                                        .and_then(|o| o.get("content").and_then(|t| t.as_str()))
                                }) {
                                    text_to_count.push_str(text);
                                }
                            }
                        }
                    }

                    if prompt_tokens.is_none() {
                        if let Some(tokens) =
                            tokenize(&selected_provider, &model, &messages.join("\n")).await
                        {
                            prompt_tokens = Some(tokens.len() as u64);
                        }
                    }
                    if completion_tokens.is_none() {
                        if let Some(tokens) =
                            tokenize(&selected_provider, &model, &text_to_count).await
                        {
                            completion_tokens = Some(tokens.len() as u64);
                        }
                    }

                    if let Some(completion_tokens) = completion_tokens {
                        let elapsed = time.elapsed();
                        let elapsed = elapsed.as_secs_f64();
                        speed = Some((completion_tokens as f64 / elapsed) as i64);
                    }

                    DB_CONNECTION.lock().await.as_ref().unwrap()
                        .execute(
                            "UPDATE requests SET response = ?1, response_time = CURRENT_TIMESTAMP, prompt_tokens = ?3, completion_tokens = ?4, speed = ?5 WHERE id = ?2",
                            params![text.clone(), id.to_string(), prompt_tokens, completion_tokens, speed],
                        )
                        .unwrap();
                    Ok(Ok(text))
                }
                Err(_) => Err(rocket::http::Status::ServiceUnavailable),
            },
            Err(_) => Err(rocket::http::Status::ServiceUnavailable),
        }
    }
}

#[get("/api/v1/models")]
async fn proxy_models(
    config: &State<SharedConfig>,
) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    let config = config.lock().await;
    let selected_provider_id = match &config.provider {
        Some(provider_id) => provider_id.clone(),
        None => return Err(rocket::http::Status::ServiceUnavailable),
    };

    let selected_provider = match config
        .providers
        .iter()
        .find(|p| p.id == selected_provider_id)
    {
        Some(provider) => provider.clone(),
        None => return Err(rocket::http::Status::ServiceUnavailable),
    };

    let api_url = format!("{}/models", selected_provider.api_url);
    let client = Client::new();

    let res = client
        .get(&api_url)
        .header(
            "Authorization",
            format!("Bearer {}", selected_provider.api_key),
        )
        .send()
        .await;

    match res {
        Ok(response) => match response.text().await {
            Ok(text) => {
                let mut response: serde_json::Map<_, _> =
                    serde_json::from_str(&text).unwrap_or_default();
                if let Some(models) = response.get_mut("data").and_then(|m| m.as_array_mut()) {
                    if let Some(override_model) = selected_provider.preset.as_ref().and_then(|p| {
                        selected_provider
                            .presets
                            .iter()
                            .find(|preset| &preset.id == p)
                            .and_then(|preset| {
                                preset.overrides.get("model").and_then(|m| m.as_str())
                            })
                    }) {
                        models.retain(|m| {
                            m.as_object()
                                .and_then(|m| m.get("id").and_then(|id| id.as_str()))
                                .map_or(true, |id| id == override_model)
                        });
                    }
                }
                Ok(Json(serde_json::Value::Object(response)))
            }
            Err(_) => Err(rocket::http::Status::ServiceUnavailable),
        },
        Err(_) => Err(rocket::http::Status::ServiceUnavailable),
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct RequestFormat {
    model: String,
}

struct Sorting {
    column: String,
    desc: bool,
}

#[get("/api/logs?<page>&<size>&<sort>")]
async fn get_logs(
    page: Option<String>,
    size: Option<String>,
    sort: Option<String>,
) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    let mut sort = sort.map(|s| {
        let mut parts = s.split(',');
        let column = parts.next().unwrap_or("timestamp").to_string();
        let desc = parts.next().map(|s| s == "desc").unwrap_or(false);
        Sorting { column, desc }
    });
    let valid_columns = [
        "timestamp",
        "provider_id",
        "prompt_tokens",
        "completion_tokens",
        "request_time",
        "response_time",
        "chat",
        "model",
        "speed",
    ];
    if let Some(ref s) = sort {
        if !valid_columns.contains(&s.column.as_str()) {
            sort = None;
        }
    }
    let db_lock = DB_CONNECTION.lock().await;
    let db = db_lock.as_ref().unwrap();
    let total_rows = db
        .prepare("SELECT COUNT(*) FROM requests")
        .unwrap()
        .query_row([], |row| row.get::<_, i64>(0))
        .unwrap();
    let mut stmt = db
        .prepare(&format!(
            "SELECT id, timestamp, provider_id, prompt_tokens, completion_tokens, request_time, response_time, chat, model, speed FROM requests ORDER BY {} {} LIMIT ?1 OFFSET ?2",
            sort.as_ref().map_or("timestamp", |s| s.column.as_str()),
            sort.as_ref().map_or("DESC", |s| if s.desc { "DESC" } else { "ASC" })
        ))
        .unwrap();
    let page_size = size.map(|s| s.parse::<i64>().unwrap_or(10)).unwrap_or(10);
    let offset = page.map(|i| i.parse::<i64>().unwrap_or(0)).unwrap_or(0) * page_size;
    let mut rows = stmt
        .query_map(params![page_size, offset], |row| {
            let id: i64 = row.get(0)?;
            let provider_id: String = row.get(2)?;
            let prompt_tokens: Option<i64> = row.get(3)?;
            let completion_tokens: Option<i64> = row.get(4)?;
            let request_time: String = row.get(5)?;
            let response_time: Option<String> = row.get(6)?;
            if response_time.is_none() {
                return Err(rusqlite::Error::InvalidQuery);
            }
            let chat: bool = row.get(7)?;
            let model: String = row.get(8)?;
            let speed: Option<i64> = row.get(9)?;
            let mut answer = HashMap::from([
                (
                    "id".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(id)),
                ),
                (
                    "provider_id".to_string(),
                    serde_json::Value::String(provider_id),
                ),
                ("model".to_string(), serde_json::Value::String(model)),
                ("chat".to_string(), serde_json::Value::Bool(chat)),
                (
                    "request_time".to_string(),
                    serde_json::Value::String(request_time),
                ),
                (
                    "response_time".to_string(),
                    serde_json::Value::String(response_time.unwrap_or("".to_string())),
                ),
            ]);
            if let Some(prompt_tokens) = prompt_tokens {
                answer.insert(
                    "prompt_tokens".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(prompt_tokens)),
                );
            }
            if let Some(completion_tokens) = completion_tokens {
                answer.insert(
                    "completion_tokens".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(completion_tokens)),
                );
            }
            if let Some(speed) = speed {
                answer.insert(
                    "speed".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(speed)),
                );
            }
            Ok(answer)
        })
        .unwrap();

    let mut logs = Vec::new();
    while let Some(row) = rows.next() {
        match row {
            Ok(row) => logs.push(row),
            Err(e) => {
                warn!("Error reading row: {:?}", e);
            }
        }
    }

    Ok(Json(json! {
        {
            "rowCount": total_rows,
            "logs": logs
        }
    }))
}

#[get("/api/logs/<id>")]
async fn get_log(id: i64) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    let db_lock = DB_CONNECTION.lock().await;
    let db = db_lock.as_ref().unwrap();
    let mut stmt = db
        .prepare(
            "SELECT id, timestamp, provider_id, chat, prompt_tokens, completion_tokens, request, response, request_time, response_time FROM requests WHERE id = ?1",
        )
        .unwrap();

    let rows = stmt.query_map([id], |row| {
        let id: i64 = row.get(0)?;
        let provider_id: String = row.get(2)?;
        let chat: bool = row.get(3)?;
        let prompt_tokens: Option<i64> = row.get(4)?;
        let completion_tokens: Option<i64> = row.get(5)?;
        let request: String = row.get(6)?;
        let response: Option<String> = row.get(7)?;
        let request_time: String = row.get(8)?;
        let response_time: Option<String> = row.get(9)?;
        let request_data: RequestFormat =
            serde_json::from_str(&request).map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut answer = HashMap::from([
            (
                "id".to_string(),
                serde_json::Value::Number(serde_json::Number::from(id)),
            ),
            (
                "provider_id".to_string(),
                serde_json::Value::String(provider_id),
            ),
            ("chat".to_string(), serde_json::Value::Bool(chat)),
            (
                "model".to_string(),
                serde_json::Value::String(request_data.model),
            ),
            (
                "request".to_string(),
                serde_json::from_str(&request).unwrap(),
            ),
            (
                "request_time".to_string(),
                serde_json::Value::String(request_time),
            ),
            (
                "response_time".to_string(),
                serde_json::Value::String(response_time.unwrap_or("".to_string())),
            ),
        ]);
        if let Some(prompt_tokens) = prompt_tokens {
            answer.insert(
                "prompt_tokens".to_string(),
                serde_json::Value::Number(serde_json::Number::from(prompt_tokens)),
            );
        }
        if let Some(completion_tokens) = completion_tokens {
            answer.insert(
                "completion_tokens".to_string(),
                serde_json::Value::Number(serde_json::Number::from(completion_tokens)),
            );
        }
        if let Some(response) = response {
            answer.insert(
                "response".to_string(),
                serde_json::from_str(&response).unwrap(),
            );
        }
        Ok(answer)
    });

    let rows = rows.unwrap().next();

    match rows {
        Some(row) => Ok(Json(json!(row.unwrap()))),
        _ => Err(rocket::http::Status::NotFound),
    }
}

#[get("/api/config/providers")]
async fn get_providers(config: &State<SharedConfig>) -> Json<Vec<ProviderConfig>> {
    let config = config.lock().await;
    Json(config.providers.clone())
}

#[get("/api/config/active-provider")]
async fn get_active_provider(config: &State<SharedConfig>) -> Json<Option<String>> {
    let config = config.lock().await;
    Json(config.provider.clone())
}

#[post("/api/config/active-provider", data = "<provider_id>")]
async fn set_active_provider(
    provider_id: String,
    config: &State<SharedConfig>,
) -> Json<HashMap<String, String>> {
    let mut config = config.lock().await;
    if provider_id.is_empty() {
        config.provider = None;
    } else if config.providers.iter().any(|p| p.id == provider_id) {
        config.provider = Some(provider_id.clone());
    } else {
        return Json(HashMap::from([(
            "message".to_string(),
            "Service not found".to_string(),
        )]));
    }
    Json(HashMap::from([(
        "message".to_string(),
        "Service updated successfully".to_string(),
    )]))
}

#[post("/api/config/providers/<provider_id>", data = "<new_provider>")]
async fn add_provider(
    provider_id: String,
    new_provider: Json<ProviderConfig>,
    config: &State<SharedConfig>,
) -> Json<HashMap<String, String>> {
    let mut config = config.lock().await;
    if config.providers.iter().any(|p| p.id == provider_id) {
        return Json(HashMap::from([(
            "message".to_string(),
            "Service already exists".to_string(),
        )]));
    }
    config.providers.push(new_provider.into_inner());
    Json(HashMap::from([(
        "message".to_string(),
        "Service added successfully".to_string(),
    )]))
}

#[put("/api/config/providers/<provider_id>", data = "<updated_provider>")]
async fn update_provider(
    provider_id: String,
    updated_provider: Json<HashMap<String, serde_json::Value>>,
    config: &State<SharedConfig>,
) -> Json<HashMap<String, String>> {
    let mut config = config.lock().await;
    if let Some(provider) = config.providers.iter_mut().find(|p| p.id == provider_id) {
        let updated_provider = updated_provider.into_inner();
        for (key, value) in updated_provider {
            match key.as_str() {
                "name" => value.as_str().map(|v| provider.name = v.to_string()),
                "api_url" => value.as_str().map(|v| provider.api_url = v.to_string()),
                "api_key" => value.as_str().map(|v| provider.api_key = v.to_string()),
                _ => None,
            };
        }
    } else {
        return Json(HashMap::from([(
            "message".to_string(),
            "Service not found".to_string(),
        )]));
    }
    Json(HashMap::from([(
        "message".to_string(),
        "Service updated successfully".to_string(),
    )]))
}

#[delete("/api/config/providers/<provider_id>")]
async fn delete_provider(
    provider_id: String,
    config: &State<SharedConfig>,
) -> Json<HashMap<String, String>> {
    let mut config = config.lock().await;
    if let Some(index) = config.providers.iter().position(|p| p.id == provider_id) {
        if config.provider == Some(provider_id) {
            config.provider = None;
        }
        config.providers.remove(index);
        Json(HashMap::from([(
            "message".to_string(),
            "Service deleted successfully".to_string(),
        )]))
    } else {
        Json(HashMap::from([(
            "message".to_string(),
            "Provider not found".to_string(),
        )]))
    }
}

#[post(
    "/api/config/providers/<provider_id>/active-preset",
    data = "<preset_id>"
)]
async fn set_active_preset(
    provider_id: String,
    preset_id: String,
    config: &State<SharedConfig>,
) -> Json<HashMap<String, String>> {
    let mut config = config.lock().await;
    if let Some(provider) = config.providers.iter_mut().find(|p| p.id == provider_id) {
        if preset_id.is_empty() {
            provider.preset = None;
            Json(HashMap::from([(
                "message".to_string(),
                "Preset removed successfully".to_string(),
            )]))
        } else if provider.presets.iter().any(|p| p.id == preset_id) {
            provider.preset = Some(preset_id);
            Json(HashMap::from([(
                "message".to_string(),
                "Preset updated successfully".to_string(),
            )]))
        } else {
            Json(HashMap::from([(
                "message".to_string(),
                "Preset not found".to_string(),
            )]))
        }
    } else {
        Json(HashMap::from([(
            "message".to_string(),
            "Provider not found".to_string(),
        )]))
    }
}

#[post("/api/config/providers/<provider_id>/presets", data = "<new_preset>")]
async fn add_preset(
    provider_id: String,
    new_preset: Json<Preset>,
    config: &State<SharedConfig>,
) -> Json<HashMap<String, String>> {
    let mut config = config.lock().await;
    if let Some(provider) = config.providers.iter_mut().find(|p| p.id == provider_id) {
        provider.presets.push(new_preset.into_inner());
        Json(HashMap::from([(
            "message".to_string(),
            "Preset added successfully".to_string(),
        )]))
    } else {
        Json(HashMap::from([(
            "message".to_string(),
            "Provider not found".to_string(),
        )]))
    }
}

#[put(
    "/api/config/providers/<provider_id>/presets/<preset_id>",
    data = "<updated_preset>"
)]
async fn update_preset(
    provider_id: String,
    preset_id: String,
    updated_preset: Json<HashMap<String, serde_json::Value>>,
    config: &State<SharedConfig>,
) -> Json<HashMap<String, String>> {
    let mut config = config.lock().await;
    if let Some(provider) = config.providers.iter_mut().find(|p| p.id == provider_id) {
        if let Some(preset) = provider.presets.iter_mut().find(|p| p.id == preset_id) {
            let updated_preset = updated_preset.into_inner();
            for (key, value) in updated_preset {
                match key.as_str() {
                    "name" => value.as_str().map(|v| preset.name = v.to_string()),
                    "overrides" => value.as_object().map(|o| {
                        for (k, v) in o {
                            preset.overrides.insert(k.to_string(), v.clone());
                        }
                    }),
                    _ => None,
                };
            }
            Json(HashMap::from([(
                "message".to_string(),
                "Preset updated successfully".to_string(),
            )]))
        } else {
            Json(HashMap::from([(
                "message".to_string(),
                "Preset not found".to_string(),
            )]))
        }
    } else {
        Json(HashMap::from([(
            "message".to_string(),
            "Provider not found".to_string(),
        )]))
    }
}

#[get("/api/config")]
async fn get_config(config: &State<SharedConfig>) -> Json<AppConfig> {
    let config = config.lock().await;
    Json(config.clone())
}

#[get("/<file..>")]
async fn index(file: PathBuf) -> Option<NamedFile> {
    let root = file.as_os_str().is_empty();
    let path = Path::new("static").join(file);
    if !root && path.exists() {
        NamedFile::open(path).await.ok()
    } else {
        NamedFile::open("static/index.html").await.ok()
    }
}

#[launch]
async fn rocket() -> _ {
    let config_path = dirs::config_dir().unwrap().join("aiswitch");
    std::fs::create_dir_all(&config_path).unwrap();
    let config_path = config_path.join("config.json");
    let config = match AppConfig::load_from_file(config_path) {
        Ok(config) => config,
        Err(_) => {
            let db_path = dirs::data_dir().unwrap().join("aiswitch").join("db.sqlite");
            let config = AppConfig {
                db_path: db_path,
                ..Default::default()
            };
            config
        }
    };
    let db_path = config.db_path.clone();
    let config = Arc::new(Mutex::new(config));

    let conn = Connection::open(db_path).unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS requests (
            id INTEGER PRIMARY KEY,
            timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            provider_id TEXT NOT NULL,
            chat BOOLEAN DEFAULT FALSE,
            request TEXT NOT NULL,
            response TEXT,
            request_time TIMESTAMP NOT NULL,
            response_time TIMESTAMP,
            prompt_tokens INTEGER,
            completion_tokens INTEGER,
            model TEXT NOT NULL,
            SPEED INTEGER
        )",
        [],
    )
    .unwrap();
    DB_CONNECTION.lock().await.replace(conn);

    let allowed_origins = AllowedOrigins::all();
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        ..Default::default()
    }
    .to_cors()
    .unwrap();
    rocket::custom(rocket::Config {
        address: "0.0.0.0".parse().unwrap(),
        port: 3400,
        ..rocket::Config::default()
    })
    .manage(config)
    .mount(
        "/",
        routes![
            index,
            proxy_completions,
            proxy_chat_completions,
            proxy_models,
            get_providers,
            get_active_provider,
            set_active_provider,
            add_provider,
            update_provider,
            delete_provider,
            set_active_preset,
            add_preset,
            update_preset,
            get_logs,
            get_log,
            get_config,
        ],
    )
    .attach(cors)
}
