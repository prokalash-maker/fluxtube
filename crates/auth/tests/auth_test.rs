use chrono::Utc;
use fluxtube_auth::{OAuthManager, TokenRefreshCoordinator};
use fluxtube_core::UserProfile;
use fluxtube_security::CredentialVault;
use fluxtube_storage::StorageManager;
use std::sync::Arc;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_token_refresh_coordinator_deduplication() {
    let mock_server = MockServer::start().await;

    // Google Token Endpoint response mock
    let token_json = serde_json::json!({
        "access_token": "ya29.dedup_test_fresh_access_token_123",
        "expires_in": 3600,
        "token_type": "Bearer"
    });

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_json))
        .expect(1) // EXACTLY 1 HTTP call must happen across all concurrent tasks!
        .mount(&mock_server)
        .await;

    let vault = CredentialVault::in_memory();
    vault
        .store_token("account_stampede_test", "refresh_secret_123")
        .unwrap();

    let client = reqwest::Client::new();
    let coordinator = Arc::new(TokenRefreshCoordinator::with_endpoint(
        vault,
        client,
        "test_client_id".to_string(),
        None,
        format!("{}/token", mock_server.uri()),
    ));

    let mut handles = Vec::new();
    for _ in 0..10 {
        let coord = coordinator.clone();
        handles.push(tokio::spawn(async move {
            coord.get_valid_access_token("account_stampede_test").await
        }));
    }

    let mut tokens = Vec::new();
    for h in handles {
        let res = h.await.unwrap().expect("Token retrieval must succeed");
        tokens.push(res);
    }

    assert_eq!(tokens.len(), 10);
    for t in tokens {
        assert_eq!(t, "ya29.dedup_test_fresh_access_token_123");
    }
}

#[tokio::test]
async fn test_multi_account_and_guest_mode() {
    let vault = CredentialVault::in_memory();
    let storage = StorageManager::open_in_memory().unwrap();
    let client = reqwest::Client::new();

    let manager = OAuthManager::new(
        vault.clone(),
        storage.clone(),
        client,
        "client_id_test".to_string(),
        None,
        vec!["https://www.googleapis.com/auth/youtube.readonly".to_string()],
    );

    // Initial state: guest mode
    assert!(manager.is_guest().unwrap());
    assert_eq!(manager.get_active_account().unwrap(), None);
    assert_eq!(manager.get_active_access_token().await.unwrap(), None);

    // Create Account 1
    let profile1 = UserProfile {
        id: "acc_1".to_string(),
        email: "alice@example.com".to_string(),
        name: "Alice".to_string(),
        picture: None,
        is_active: true,
        created_at: Utc::now(),
    };
    vault.store_token("acc_1", "refresh_alice").unwrap();
    manager.storage().save_account(&profile1, true).unwrap();
    manager
        .coordinator()
        .store_tokens("acc_1", "access_alice".to_string(), 3600, None)
        .unwrap();

    // Verify Account 1 is active
    assert!(!manager.is_guest().unwrap());
    assert_eq!(manager.get_active_account().unwrap().unwrap().id, "acc_1");
    assert_eq!(
        manager.get_active_access_token().await.unwrap().unwrap(),
        "access_alice"
    );

    // Create Account 2
    let profile2 = UserProfile {
        id: "acc_2".to_string(),
        email: "bob@example.com".to_string(),
        name: "Bob".to_string(),
        picture: None,
        is_active: false,
        created_at: Utc::now(),
    };
    vault.store_token("acc_2", "refresh_bob").unwrap();
    manager.storage().save_account(&profile2, false).unwrap();
    manager
        .coordinator()
        .store_tokens("acc_2", "access_bob".to_string(), 3600, None)
        .unwrap();

    let accounts = manager.list_accounts().unwrap();
    assert_eq!(accounts.len(), 2);

    // Switch to Account 2
    manager.switch_account(Some("acc_2")).unwrap();
    assert_eq!(manager.get_active_account().unwrap().unwrap().id, "acc_2");
    assert_eq!(
        manager.get_active_access_token().await.unwrap().unwrap(),
        "access_bob"
    );

    // Switch to Guest Mode
    manager.switch_account(None).unwrap();
    assert!(manager.is_guest().unwrap());
    assert_eq!(manager.get_active_account().unwrap(), None);
    assert_eq!(manager.get_active_access_token().await.unwrap(), None);

    // Switch back to Account 1 then logout
    manager.switch_account(Some("acc_1")).unwrap();
    assert_eq!(manager.get_active_account().unwrap().unwrap().id, "acc_1");

    manager.logout("acc_1").await.unwrap();
    assert_eq!(vault.get_token("acc_1").unwrap(), None);
    let accounts_after = manager.list_accounts().unwrap();
    assert_eq!(accounts_after.len(), 1);
    assert_eq!(accounts_after[0].id, "acc_2");
}
