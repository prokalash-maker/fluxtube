use fluxtube_network::{
    BackoffConfig, CircuitBreakerConfig, CircuitState, ResilientHttpClientBuilder,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_resilient_client_retries_transient_failures() {
    let mock_server = MockServer::start().await;

    // Fail twice with 503, then succeed on 3rd attempt with 200
    Mock::given(method("GET"))
        .and(path("/retry-test"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .expect(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/retry-test"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"recovered payload"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = ResilientHttpClientBuilder::default()
        .backoff(BackoffConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(50),
            multiplier: 1.5,
            jitter_factor: 0.1,
        })
        .build()
        .unwrap();

    let url = format!("{}/retry-test", mock_server.uri());
    let bytes = client.get_bytes(&url).await.expect("should succeed after retries");

    assert_eq!(&bytes[..], b"recovered payload");
}

#[tokio::test]
async fn test_resilient_client_circuit_breaker_trips() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/always-fail"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = ResilientHttpClientBuilder::default()
        .backoff(BackoffConfig {
            max_retries: 0,
            initial_delay: Duration::from_millis(5),
            max_delay: Duration::from_millis(10),
            multiplier: 1.0,
            jitter_factor: 0.0,
        })
        .circuit_breaker(CircuitBreakerConfig {
            failure_threshold: 2,
            cool_off_duration: Duration::from_millis(500),
            half_open_success_threshold: 1,
        })
        .build()
        .unwrap();

    let url = format!("{}/always-fail", mock_server.uri());

    // 1st failure
    let _ = client.get_bytes(&url).await;
    assert_eq!(client.circuit_breaker().state(), CircuitState::Closed);

    // 2nd failure -> trips circuit
    let _ = client.get_bytes(&url).await;
    assert_eq!(client.circuit_breaker().state(), CircuitState::Open);

    // 3rd attempt -> blocked by circuit breaker before network call
    let err = client.get_bytes(&url).await.unwrap_err();
    match err {
        fluxtube_network::NetworkError::CircuitBreakerOpen { .. } => {}
        other => panic!("Expected CircuitBreakerOpen, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_resilient_client_deduplication_wiremock() {
    let mock_server = MockServer::start().await;
    let hit_count = Arc::new(AtomicUsize::new(0));

    Mock::given(method("GET"))
        .and(path("/dedup-test"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"deduplicated response"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = Arc::new(
        ResilientHttpClientBuilder::default()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap(),
    );

    let url = format!("{}/dedup-test", mock_server.uri());
    let mut handles = Vec::new();

    for _ in 0..8 {
        let c = client.clone();
        let u = url.clone();
        let hc = hit_count.clone();
        handles.push(tokio::spawn(async move {
            let res = c.get_bytes(&u).await.unwrap();
            hc.fetch_add(1, Ordering::SeqCst);
            res
        }));
    }

    for h in handles {
        let b = h.await.unwrap();
        assert_eq!(&b[..], b"deduplicated response");
    }

    assert_eq!(hit_count.load(Ordering::SeqCst), 8);
}
