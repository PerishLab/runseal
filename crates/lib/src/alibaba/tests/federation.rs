use super::{Fixture, base64};
use crate::alibaba::{
    Auth, Binding, Outcome,
    auth::role,
    model::{self, Pending, Retained, State},
};
use radicula_oidc::jwt::{Algorithm, Audience, Claims, EncodingKey, Signer};
use serde_json::{Value, json};

fn binding() -> Binding {
    Binding {
        source: "https://identity.example/api".into(),
        issuer: "https://identity.example/api".into(),
        subject: "external-subject".into(),
        audience: "ali-client".into(),
        account: "1234567890".into(),
        provider: "acs:ram::1234567890:oidc-provider/identity".into(),
        role: "acs:ram::1234567890:role/operator".into(),
        id: "2345678901".into(),
        session: "fixture-session".into(),
        duration: 900,
    }
}

fn claims() -> Claims {
    Claims {
        iss: binding().issuer,
        sub: binding().subject,
        aud: Audience::One(binding().audience),
        iat: model::now() - 10,
        exp: model::now() + 300,
        nbf: None,
        extra: serde_json::Map::from_iter([("kind".into(), json!("access"))]),
    }
}

fn signed(claims: &Claims) -> Value {
    use base64::Engine;
    use p256::elliptic_curve::sec1::ToEncodedPoint;
    use p256::pkcs8::{EncodePrivateKey, LineEnding};
    let secret = p256::SecretKey::from_slice(&[7; 32]).unwrap();
    let pem = secret.to_pkcs8_pem(LineEnding::LF).unwrap();
    let point = secret.public_key().to_encoded_point(false);
    let encode = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let signer = Signer {
        algorithm: Algorithm::ES256,
        kid: "fixture".into(),
        key: EncodingKey::from_ec_pem(pem.as_bytes()).unwrap(),
    };
    json!({"token":signer.sign(claims).unwrap().expose(),"keys":{"keys":[{"kid":"fixture","alg":"ES256","use":"sig","kty":"EC","crv":"P-256","x":encode.encode(point.x().unwrap()),"y":encode.encode(point.y().unwrap())}]}})
}

fn state() -> State {
    let mut state = State::fresh();
    state.version = 2;
    state.federation = Some(role::Federation {
        binding: binding(),
        token: Some(role::Token {
            value: "PRIVATE_UPSTREAM_MARKER".into(),
            expires: 1700003600,
            issued: 1700000000,
        }),
        identity: None,
    });
    state
}

fn fixture(stage: &str, status: u16, body: &Value) -> Fixture {
    let fixture = Fixture::new();
    let mut state = state();
    state.pending = Some(Pending {
        stage: stage.into(),
        started: model::now(),
    });
    state.retained = Some(Retained {
        stage: stage.into(),
        status,
        body: base64(&serde_json::to_vec(body).unwrap()),
    });
    fixture.store().create(&state).unwrap();
    fixture
}

fn timestamp(value: u64) -> String {
    crate::alibaba::api::sign::date(value).unwrap()
}

fn returned() -> Value {
    let now = model::now();
    json!({"RequestId":"fixture-id","OIDCTokenInfo":{"Issuer":binding().issuer,"Subject":binding().subject,"ClientIds":binding().audience,"VerificationInfo":"Success","ExpirationTime":timestamp(1700003600),"IssuanceTime":timestamp(1700000000)},"AssumedRoleUser":{"Arn":"acs:ram::1234567890:role/operator/fixture-session","AssumedRoleId":"opaque-session-id"},"Credentials":{"AccessKeyId":"STS.fixture","AccessKeySecret":"PRIVATE_KEY_MARKER","SecurityToken":"PRIVATE_TOKEN_MARKER","Expiration":timestamp(now+800)}})
}

#[test]
fn upstream() {
    let body = signed(&claims());
    let fixture = fixture(model::OIDC, 200, &body);
    let auth = Auth::new(&fixture.state).unwrap();
    let receipt = auth.recover().unwrap();
    assert_eq!(receipt["federation"]["tokenPresent"], true);
    assert_eq!(receipt["credentialPresent"], false);
    assert!(
        !receipt
            .to_string()
            .contains(body["token"].as_str().unwrap())
    );
    assert!(receipt["pending"].is_null());
}

#[test]
fn claimsrefused() {
    for key in ["issuer", "subject", "audience", "kind", "expired", "future"] {
        let mut claims = claims();
        match key {
            "issuer" => claims.iss = "https://foreign.example/api".into(),
            "subject" => claims.sub = "another-subject".into(),
            "audience" => claims.aud = Audience::Many(vec![binding().audience, "foreign".into()]),
            "kind" => {
                claims.extra.insert("kind".into(), json!("id"));
            }
            "expired" => {
                claims.iat = model::now() - 100;
                claims.exp = model::now() - 10;
            }
            _ => claims.iat = model::now() + 5,
        }
        let body = signed(&claims);
        let fixture = fixture(model::OIDC, 200, &body);
        let auth = Auth::new(&fixture.state).unwrap();
        assert_eq!(
            auth.recover().unwrap_err().outcome,
            Outcome::Unknown,
            "{key}"
        );
        assert!(
            auth.status().unwrap()["pending"]["retainedReturn"]
                .as_bool()
                .unwrap()
        );
    }
}

#[test]
fn signatures() {
    for key in ["signature", "kid", "duplicate", "algorithm"] {
        let mut body = signed(&claims());
        match key {
            "signature" => {
                let token = body["token"].as_str().unwrap().to_owned() + "x";
                body["token"] = json!(token);
            }
            "kid" => body["keys"]["keys"][0]["kid"] = json!("foreign"),
            "duplicate" => {
                let second = body["keys"]["keys"][0].clone();
                body["keys"]["keys"].as_array_mut().unwrap().push(second);
            }
            _ => body["keys"]["keys"][0]["alg"] = json!("HS256"),
        }
        let fixture = fixture(model::OIDC, 200, &body);
        assert_eq!(
            Auth::new(&fixture.state)
                .unwrap()
                .recover()
                .unwrap_err()
                .outcome,
            Outcome::Unknown
        );
    }
}

#[test]
fn exchange() {
    let fixture = fixture(model::ROLE, 200, &returned());
    let auth = Auth::new(&fixture.state).unwrap();
    let receipt = auth.recover().unwrap();
    assert_eq!(receipt["credentialPresent"], true);
    assert!(receipt["federation"]["identity"].is_null());
    assert!(!receipt.to_string().contains("PRIVATE_"));
}

#[test]
fn foreign() {
    for (part, key) in [
        ("OIDCTokenInfo", "Issuer"),
        ("OIDCTokenInfo", "Subject"),
        ("OIDCTokenInfo", "ClientIds"),
        ("OIDCTokenInfo", "VerificationInfo"),
        ("OIDCTokenInfo", "ExpirationTime"),
        ("AssumedRoleUser", "Arn"),
        ("Credentials", "Expiration"),
    ] {
        let mut body = returned();
        body[part][key] = json!("foreign");
        let fixture = fixture(model::ROLE, 200, &body);
        let auth = Auth::new(&fixture.state).unwrap();
        assert_eq!(
            auth.recover().unwrap_err().outcome,
            Outcome::Unknown,
            "{part}/{key}"
        );
        assert_eq!(auth.status().unwrap()["pending"]["retainedReturn"], true);
    }
}

#[test]
fn uncertainty() {
    for status in [400, 403, 500] {
        let fixture = fixture(
            model::ROLE,
            status,
            &json!({"Code":"PRIVATE_UNRECOGNIZED","Message":"PRIVATE_MARKER"}),
        );
        let auth = Auth::new(&fixture.state).unwrap();
        let error = auth.recover().unwrap_err();
        assert_eq!(error.outcome, Outcome::Unknown);
        assert!(!error.to_string().contains("PRIVATE_"));
        assert!(auth.renew().is_err());
    }
    let fixture = fixture(
        model::ROLE,
        403,
        &json!({"Code":"Forbidden.RAM","RequestId":"fixture-id"}),
    );
    let auth = Auth::new(&fixture.state).unwrap();
    assert_eq!(auth.recover().unwrap_err().outcome, Outcome::Rejected);
    assert!(auth.status().unwrap()["pending"].is_null());
}

#[test]
fn modes() {
    let mut state = state();
    state.check().unwrap();
    state.version = 1;
    assert!(state.check().is_err());
    state.version = 2;
    state.identity = Some(super::identity());
    assert!(state.check().is_err());
    state.identity = None;
    state.pending = Some(Pending {
        stage: model::STS.into(),
        started: model::now(),
    });
    assert!(state.check().is_err());
    let fixture = Fixture::new();
    fixture.store().create(&self::state()).unwrap();
    assert!(Auth::new(&fixture.state).unwrap().begin().is_err());
}

#[test]
fn caller() {
    let held = state().federation.unwrap();
    let body = json!({"AccountId":binding().account,"PrincipalId":"opaque-session-principal","RoleId":binding().id,"IdentityType":"AssumedRoleUser","Arn":"acs:ram::1234567890:assumed-role/operator/fixture-session","RequestId":"fixture-id"});
    assert_eq!(
        role::decode(&body, &held, "stamp").unwrap().role,
        binding().id
    );
    for key in ["AccountId", "RoleId", "Arn", "IdentityType"] {
        let mut foreign = body.clone();
        foreign[key] = json!("foreign");
        assert!(role::decode(&foreign, &held, "stamp").is_err());
    }
    let mut missing = body.clone();
    missing.as_object_mut().unwrap().remove("RoleId");
    missing["UserId"] = json!(binding().id);
    assert!(role::decode(&missing, &held, "stamp").is_err());
}

#[test]
fn declarations() {
    binding().validate().unwrap();
    for key in [
        "source", "issuer", "account", "provider", "role", "id", "session", "duration",
    ] {
        let mut body = serde_json::to_value(binding()).unwrap();
        body[key] = if key == "duration" {
            json!(0)
        } else {
            json!("foreign/session")
        };
        assert!(
            serde_json::from_value::<Binding>(body)
                .unwrap()
                .validate()
                .is_err(),
            "{key}"
        );
    }
    let mut alias = binding();
    alias.source = "http://127.0.0.1:9000/api".into();
    alias.validate().unwrap();
    alias.issuer = alias.source.clone();
    assert!(alias.validate().is_err());
}
