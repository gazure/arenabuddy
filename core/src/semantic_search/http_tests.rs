use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
};

use super::*;

struct MockApi {
    mode: &'static str,
    status: StatusCode,
    requests: Mutex<Vec<Value>>,
}

async fn respond(
    State(state): State<Arc<MockApi>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    assert_eq!(headers["authorization"], "Bearer fake-test-key");
    state.requests.lock().unwrap().push(body.clone());
    let answers: BTreeMap<_, _> = body["questions"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(id, question)| {
            let answer = match question["type"].as_str().unwrap() {
                "score" => {
                    let index: usize = id.strip_prefix("card_").unwrap().parse().unwrap();
                    json!({"type": "score", "score": ([1.0, 3.0, 2.0][index])})
                }
                "noul" => json!({"type": "noul", "noul": 0.01}),
                _ => {
                    let choice = match id.as_str() {
                        "mode" => state.mode,
                        "color_mode" => "unspecified",
                        _ => "none",
                    };
                    json!({"type": "choice", "choice": choice, "confidence": 0.99})
                }
            };
            (id.clone(), answer)
        })
        .collect();
    (
        state.status,
        Json(json!({"answers": answers, "usage": {"input_tokens": 10, "output_tokens": 5}})),
    )
}

struct Server {
    endpoint: String,
    state: Arc<MockApi>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Server {
    async fn new(mode: &'static str, status: StatusCode) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/evaluate", listener.local_addr().unwrap());
        let state = Arc::new(MockApi {
            mode,
            status,
            requests: Mutex::new(Vec::new()),
        });
        let app = Router::new()
            .route("/evaluate", post(respond))
            .with_state(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self { endpoint, state, task }
    }
}

#[tokio::test]
async fn ranks_local_cards_and_aggregates_usage_without_sending_key_in_json() {
    let server = Server::new("semantic", StatusCode::OK).await;
    let result = search_with_endpoint(
        &tests::fixture(),
        &SearchOptions::for_query("creatures".into()),
        "fake-test-key",
        &reqwest::Client::new(),
        &server.endpoint,
    )
    .await
    .unwrap();
    assert_eq!(
        result.matches.iter().map(|found| found.card.id).collect::<Vec<_>>(),
        vec![1, 4]
    );
    assert_eq!(result.matches[0].score, Some(3.0));
    assert_eq!(result.ranked_count, 3);
    assert_eq!(result.usage.input_tokens, 20);
    assert_eq!(result.usage.output_tokens, 10);
    let requests = server.state.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(!requests.iter().any(|body| body.to_string().contains("fake-test-key")));
    assert_eq!(requests[1]["state"]["cards"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn filter_only_search_applies_overrides_and_skips_ranking() {
    let server = Server::new("filters", StatusCode::OK).await;
    let mut options = SearchOptions::for_query("creatures".into());
    options.format = Some("standard".into());
    options.max_mana_value = Some(3);
    let result = search_with_endpoint(
        &tests::fixture(),
        &options,
        "fake-test-key",
        &reqwest::Client::new(),
        &server.endpoint,
    )
    .await
    .unwrap();
    assert!(!result.ranked);
    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].card.id, 1);
    assert_eq!(result.matches[0].score, None);
    assert!(result.filter_summary.contains(&"Mana value ≤ 3".to_string()));
    assert_eq!(server.state.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn surfaces_authentication_and_rate_limit_errors() {
    for status in [
        StatusCode::UNAUTHORIZED,
        StatusCode::FORBIDDEN,
        StatusCode::TOO_MANY_REQUESTS,
    ] {
        let server = Server::new("filters", status).await;
        let error = search_with_endpoint(
            &tests::fixture(),
            &SearchOptions::for_query("creatures".into()),
            "fake-test-key",
            &reqwest::Client::new(),
            &server.endpoint,
        )
        .await
        .unwrap_err();
        if status == StatusCode::TOO_MANY_REQUESTS {
            assert!(matches!(error, Error::RateLimited));
        } else {
            assert!(matches!(error, Error::Authentication));
        }
    }
}

#[tokio::test]
async fn invalid_input_never_sends_a_request() {
    let server = Server::new("filters", StatusCode::OK).await;
    let client = reqwest::Client::new();
    let mut options = SearchOptions::for_query(" ".into());
    assert!(
        search_with_endpoint(&tests::fixture(), &options, "fake-test-key", &client, &server.endpoint)
            .await
            .is_err()
    );
    options.query = "creatures".into();
    options.candidates = 0;
    assert!(
        search_with_endpoint(&tests::fixture(), &options, "fake-test-key", &client, &server.endpoint)
            .await
            .is_err()
    );
    options.candidates = 60;
    assert!(matches!(
        search_with_endpoint(&tests::fixture(), &options, "", &client, &server.endpoint).await,
        Err(Error::Authentication)
    ));
    assert!(server.state.requests.lock().unwrap().is_empty());
}
