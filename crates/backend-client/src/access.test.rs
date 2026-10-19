use super::*;

fn headers_of(credential: &AccessCredential) -> http::HeaderMap {
    let mut headers = http::HeaderMap::new();
    credential.apply(&mut headers);
    headers
}

#[test]
fn a_user_token_is_sent_as_cf_access_token() {
    let headers = headers_of(&AccessCredential::UserToken("user-jwt".to_string()));
    assert_eq!(headers.len(), 1);
    assert_eq!(headers["cf-access-token"], "user-jwt");
    assert!(headers["cf-access-token"].is_sensitive());
}

#[test]
fn a_service_token_is_sent_as_the_client_pair() {
    let headers = headers_of(&AccessCredential::ServiceToken {
        client_id: "id.access".to_string(),
        client_secret: "secret".to_string(),
    });
    assert_eq!(headers.len(), 2);
    assert_eq!(headers[ACCESS_CLIENT_ID_HEADER], "id.access");
    assert_eq!(headers[ACCESS_CLIENT_SECRET_HEADER], "secret");
    assert!(headers[ACCESS_CLIENT_ID_HEADER].is_sensitive());
    assert!(headers[ACCESS_CLIENT_SECRET_HEADER].is_sensitive());
}

#[test]
fn the_local_identity_is_sent_as_the_authenticated_user_email() {
    let headers = headers_of(&AccessCredential::LocalIdentity("dev@local".to_string()));
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[LOCAL_IDENTITY_HEADER], "dev@local");
}

#[test]
fn a_value_that_is_not_a_header_value_is_left_out() {
    let headers = headers_of(&AccessCredential::UserToken("bad\nvalue".to_string()));
    assert!(headers.is_empty());
}

#[test]
fn debug_never_prints_a_value() {
    for credential in [
        AccessCredential::UserToken("user-jwt".to_string()),
        AccessCredential::ServiceToken {
            client_id: "id.access".to_string(),
            client_secret: "secret".to_string(),
        },
        AccessCredential::LocalIdentity("dev@local".to_string()),
    ] {
        let printed = format!("{credential:?}");
        for value in ["user-jwt", "id.access", "secret", "dev@local"] {
            assert!(!printed.contains(value), "{printed}");
        }
        assert!(printed.contains("<redacted>"), "{printed}");
    }
}
