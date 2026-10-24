//! Registered models and their versions, and what catalogs, schemas and
//! models hold (`Contents`).

use futures::TryStreamExt;

use uc_client::Contents;
use uc_client::models::models::v1::ModelVersionStatus;
use uc_client::models::volumes::v1::VolumeType;

use crate::common::{catalog_and_schema, client, rejected};

#[tokio::test]
async fn registered_models() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let created = uc
        .create_registered_model("m", "c", "s")
        .with_comment("first".to_string())
        .await
        .unwrap();
    assert_eq!(created.full_name, "c.s.m");
    assert_eq!(created.comment.as_deref(), Some("first"));
    assert!(!created.storage_location.is_empty() && !created.id.is_empty());
    assert_eq!(uc.registered_model("c.s.m").get().await.unwrap(), created);
    uc.create_schema("t", "c").await.unwrap();
    uc.create_registered_model("n", "c", "t").await.unwrap();
    let listed = uc
        .list_registered_models()
        .with_catalog_name("c".to_string())
        .with_schema_name("s".to_string())
        .await
        .unwrap();
    assert_eq!(listed.registered_models, std::slice::from_ref(&created));
    let first = uc
        .list_registered_models()
        .with_max_results(1)
        .await
        .unwrap();
    let second = uc
        .list_registered_models()
        .with_max_results(1)
        .with_page_token(first.next_page_token.clone())
        .await
        .unwrap();
    let mut names: Vec<_> = [first, second]
        .into_iter()
        .flat_map(|page| page.registered_models)
        .map(|m| m.full_name)
        .collect();
    names.sort();
    assert_eq!(names, ["c.s.m", "c.t.n"]);

    let updated = uc
        .registered_model("c.s.m")
        .update()
        .with_comment("edited".to_string())
        .with_new_name("m2".to_string())
        .await
        .unwrap();
    assert_eq!(updated.full_name, "c.s.m2");
    assert_eq!(updated.comment.as_deref(), Some("edited"));
    assert_eq!(updated.id, created.id);

    uc.registered_model("c.s.m2").delete().await.unwrap();
    rejected(uc.registered_model("c.s.m2").get().await, 404, "NOT_FOUND");
}

#[tokio::test]
async fn model_versions() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    uc.create_registered_model("m", "c", "s").await.unwrap();
    let version = uc
        .create_model_version("m", "c", "s", "s3://bucket/m/v1")
        .with_run_id("run-1".to_string())
        .with_comment("v1".to_string())
        .await
        .unwrap();
    assert_eq!(version.version, 1);
    assert_eq!(version.source, "s3://bucket/m/v1");
    assert_eq!(version.run_id.as_deref(), Some("run-1"));
    assert_eq!(version.comment.as_deref(), Some("v1"));
    assert_eq!(version.status, ModelVersionStatus::PendingRegistration);
    assert!(!version.storage_location.is_empty());
    assert_eq!(uc.get_model_version("c.s.m", 1).await.unwrap(), version);

    let edited = uc
        .update_model_version("c.s.m", 1, "v1, edited")
        .await
        .unwrap();
    assert_eq!(edited.comment.as_deref(), Some("v1, edited"));
    let ready = uc.finalize_model_version("c.s.m", 1).await.unwrap();
    assert_eq!(ready.status, ModelVersionStatus::Ready);

    let second = uc
        .create_model_version("m", "c", "s", "s3://bucket/m/v2")
        .await
        .unwrap();
    assert_eq!(second.version, 2);
    let first = uc
        .list_model_versions("c.s.m")
        .with_max_results(1)
        .await
        .unwrap();
    assert_eq!(first.model_versions, std::slice::from_ref(&ready));
    let next = uc
        .list_model_versions("c.s.m")
        .with_max_results(1)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    assert_eq!(next.model_versions, [second]);
    uc.delete_model_version("c.s.m", 2).await.unwrap();
    rejected(uc.get_model_version("c.s.m", 2).await, 404, "NOT_FOUND");
    let all: Vec<_> = uc
        .list_model_versions("c.s.m")
        .into_stream()
        .try_collect()
        .await
        .unwrap();
    assert_eq!(all, [ready]);

    // A model with versions goes only with `force`.
    rejected(uc.registered_model("c.s.m").delete().await, 409, "ABORTED");
    uc.registered_model("c.s.m")
        .delete()
        .with_force(true)
        .await
        .unwrap();
    assert!(
        uc.list_registered_models()
            .await
            .unwrap()
            .registered_models
            .is_empty()
    );
}

#[tokio::test]
async fn what_catalogs_schemas_and_models_hold() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    for name in ["v1", "v2"] {
        uc.create_volume("c", "s", name, VolumeType::Managed)
            .await
            .unwrap();
    }
    uc.create_registered_model("m", "c", "s").await.unwrap();
    for run in ["1", "2"] {
        uc.create_model_version("m", "c", "s", format!("s3://bucket/m/v{run}"))
            .await
            .unwrap();
    }
    uc.create_schema("empty", "c").await.unwrap();

    let schema = Contents {
        volumes: 2,
        models: 1,
        versions: 2,
        ..Contents::default()
    };
    assert_eq!(uc.schema_contents("c", "s").await.unwrap(), schema);
    assert_eq!(
        uc.schema_contents("c", "empty").await.unwrap(),
        Contents::default()
    );
    assert_eq!(
        uc.catalog_contents("c").await.unwrap(),
        Contents {
            schemas: 2,
            ..schema
        }
    );
    assert_eq!(
        uc.model_contents("c.s.m").await.unwrap(),
        Contents {
            versions: 2,
            ..Contents::default()
        }
    );
}

/// The comment is the only field of a model version that changes, so an
/// empty one clears it. The client leaves an empty `comment` out of the
/// body, and the server answers 400 `No updated fields defined.`.
#[tokio::test]
async fn a_model_version_comment_can_be_cleared() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    uc.create_registered_model("m", "c", "s").await.unwrap();
    uc.create_model_version("m", "c", "s", "s3://bucket/m/v1")
        .with_comment("v1".to_string())
        .await
        .unwrap();
    let cleared = uc.update_model_version("c.s.m", 1, "").await.unwrap();
    assert_eq!(cleared.comment.unwrap_or_default(), "");
}
