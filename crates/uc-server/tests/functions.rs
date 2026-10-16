mod common;

use axum::{
    Router,
    http::{Method, StatusCode},
};
use rstest::rstest;
use serde_json::json;
use tower::ServiceExt;

use common::{app, assert_status, json_body, req, req_json, seeded};

fn minimal_fn(name: &str) -> serde_json::Value {
    // Upstream wraps the payload in `{"function_info": {...}}`.
    json!({
        "function_info": {
            "name": name,
            "catalog_name": "main",
            "schema_name": "default",
            "input_params": {
                "parameters": [
                    {
                        "name": "x", "type_text": "long", "type_json": "\"long\"",
                        "type_name": "LONG", "position": 0,
                    }
                ]
            },
            "data_type": "LONG",
            "full_data_type": "long",
            "routine_body": "SQL",
            "routine_definition": "x + 1",
            "parameter_style": "S",
            "is_deterministic": true,
            "sql_data_access": "NO_SQL",
            "is_null_call": false,
            "security_type": "DEFINER",
            "specific_name": name,
        }
    })
}

#[rstest]
#[tokio::test]
async fn create_get_list_delete_function(#[future(awt)] seeded: Router) {
    let resp = seeded
        .clone()
        .oneshot(req_json(Method::POST, "/functions", minimal_fn("inc")))
        .await
        .unwrap();
    let info = json_body(assert_status(resp, StatusCode::OK, "create fn").await).await;
    assert_eq!(info["name"], "inc");
    assert_eq!(info["full_name"], "main.default.inc");
    assert_eq!(info["routine_body"], "SQL");
    assert_eq!(info["data_type"], "LONG");
    assert!(info["function_id"].is_string());

    let resp = seeded
        .clone()
        .oneshot(req(Method::GET, "/functions/main.default.inc"))
        .await
        .unwrap();
    let _ = assert_status(resp, StatusCode::OK, "get").await;

    let resp = seeded
        .clone()
        .oneshot(req(
            Method::GET,
            "/functions?catalog_name=main&schema_name=default",
        ))
        .await
        .unwrap();
    let list = json_body(assert_status(resp, StatusCode::OK, "list").await).await;
    assert_eq!(list["functions"].as_array().unwrap().len(), 1);

    let resp = seeded
        .oneshot(req(Method::DELETE, "/functions/main.default.inc"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[rstest]
#[tokio::test]
async fn function_under_missing_schema_returns_404(#[future(awt)] app: Router) {
    app.clone()
        .oneshot(req_json(
            Method::POST,
            "/catalogs",
            json!({ "name": "main" }),
        ))
        .await
        .unwrap();
    let mut body = minimal_fn("x");
    body["function_info"]["schema_name"] = json!("ghost");

    let resp = app
        .oneshot(req_json(Method::POST, "/functions", body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Upstream stores a function with only the enum fields it needs; the spec's
/// other "required" fields may be absent.
#[rstest]
#[tokio::test]
async fn optional_fields_may_be_absent(#[future(awt)] seeded: Router) {
    let body = json!({ "function_info": {
        "name": "bare", "catalog_name": "main", "schema_name": "default",
        "routine_body": "EXTERNAL", "parameter_style": "S", "security_type": "DEFINER",
    }});
    let resp = seeded
        .clone()
        .oneshot(req_json(Method::POST, "/functions", body))
        .await
        .unwrap();
    let info = json_body(assert_status(resp, StatusCode::OK, "create bare").await).await;
    assert!(info.get("input_params").is_none());
    assert!(info.get("data_type").is_none());

    // A parameter without type_json or position.
    let mut with_param = minimal_fn("loose");
    let param = &mut with_param["function_info"]["input_params"]["parameters"][0];
    param.as_object_mut().unwrap().remove("type_json");
    param.as_object_mut().unwrap().remove("position");
    let resp = seeded
        .clone()
        .oneshot(req_json(Method::POST, "/functions", with_param))
        .await
        .unwrap();
    assert_status(resp, StatusCode::OK, "create loose").await;

    let resp = seeded
        .clone()
        .oneshot(req(Method::GET, "/functions/main.default.loose"))
        .await
        .unwrap();
    let got = json_body(assert_status(resp, StatusCode::OK, "get loose").await).await;
    assert_eq!(got["input_params"]["parameters"][0]["name"], "x");
    assert!(
        got["input_params"]["parameters"][0]
            .get("type_json")
            .is_none()
    );

    let mut nameless = minimal_fn("x");
    nameless["function_info"]
        .as_object_mut()
        .unwrap()
        .remove("name");
    let resp = seeded
        .oneshot(req_json(Method::POST, "/functions", nameless))
        .await
        .unwrap();
    let err = common::assert_error(resp, StatusCode::BAD_REQUEST, "INVALID_ARGUMENT").await;
    assert_eq!(err["message"], "Name cannot be empty");
}
