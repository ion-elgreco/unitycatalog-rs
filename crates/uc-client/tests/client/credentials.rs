//! Credentials and external locations, and the deletes that need `force`.

use futures::TryStreamExt;

use uc_client::models::credentials::v1::{AwsIamRoleRequest, CredentialPurpose};
use uc_client::models::volumes::v1::VolumeType;

use crate::common::{catalog_and_schema, client, rejected};

const ROLE: &str = "arn:aws:iam::123456789012:role/uc";

fn role(arn: &str) -> AwsIamRoleRequest {
    AwsIamRoleRequest {
        role_arn: arn.into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn credentials() {
    let uc = client().await;
    let credential = uc
        .create_credential("cred", role(ROLE))
        .with_comment("prod".to_string())
        .with_purpose(CredentialPurpose::Storage)
        .await
        .unwrap();
    assert_eq!(credential.name, "cred");
    assert_eq!(credential.purpose, Some(CredentialPurpose::Storage.into()));
    assert_eq!(credential.comment.as_deref(), Some("prod"));
    let aws = credential.aws_iam_role.as_option().unwrap();
    assert_eq!(aws.role_arn, ROLE);
    assert!(!aws.external_id.is_empty());
    assert_eq!(uc.credential("cred").get().await.unwrap(), credential);
    uc.create_credential("service", role(ROLE))
        .with_purpose(CredentialPurpose::ServicePrincipal)
        .await
        .unwrap();
    let storage = uc
        .list_credentials()
        .with_purpose(CredentialPurpose::Storage)
        .await
        .unwrap();
    assert_eq!(storage.credentials, std::slice::from_ref(&credential));
    let first = uc.list_credentials().with_max_results(1).await.unwrap();
    assert_eq!(first.credentials, std::slice::from_ref(&credential));
    let second = uc
        .list_credentials()
        .with_max_results(1)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    assert_eq!(second.credentials[0].name, "service");

    let new_role = format!("{ROLE}2");
    let updated = uc
        .credential("cred")
        .update()
        .with_comment("edited".to_string())
        .with_aws_iam_role(role(&new_role))
        .with_new_name("cred2".to_string())
        .await
        .unwrap();
    assert_eq!(updated.name, "cred2");
    assert_eq!(updated.comment.as_deref(), Some("edited"));
    let aws = updated.aws_iam_role.as_option().unwrap();
    assert_eq!(aws.role_arn, new_role);
    assert_ne!(aws.external_id, credential.aws_iam_role.external_id);

    uc.credential("cred2").delete().await.unwrap();
    rejected(uc.credential("cred2").get().await, 404, "NOT_FOUND");
}

/// A credential that a location uses, and a location that a volume uses,
/// go only with `force`.
#[tokio::test]
async fn external_locations() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let credential = uc.create_credential("cred", role(ROLE)).await.unwrap();
    uc.create_credential("other", role(ROLE)).await.unwrap();
    let location = uc
        .create_external_location("loc", "s3://bucket/loc", "cred")
        .with_comment("lake".to_string())
        .await
        .unwrap();
    assert_eq!(location.url, "s3://bucket/loc");
    assert_eq!(location.credential_name.as_deref(), Some("cred"));
    assert_eq!(location.credential_id, credential.id);
    assert_eq!(location.comment.as_deref(), Some("lake"));
    assert_eq!(uc.external_location("loc").get().await.unwrap(), location);
    let listed: Vec<_> = uc
        .list_external_locations()
        .into_stream()
        .try_collect()
        .await
        .unwrap();
    assert_eq!(listed, std::slice::from_ref(&location));

    let moved = uc
        .external_location("loc")
        .update()
        .with_url("s3://bucket/lake".to_string())
        .with_new_name("loc2".to_string())
        .with_comment("edited".to_string())
        .with_credential_name("other".to_string())
        .await
        .unwrap();
    assert_eq!(moved.name, "loc2");
    assert_eq!(moved.url, "s3://bucket/lake");
    assert_eq!(moved.comment.as_deref(), Some("edited"));
    assert_eq!(moved.credential_name.as_deref(), Some("other"));
    let page = uc
        .list_external_locations()
        .with_max_results(1)
        .await
        .unwrap();
    assert_eq!(page.external_locations, [moved]);

    let volume = uc
        .create_volume("c", "s", "v", VolumeType::External)
        .with_storage_location("s3://bucket/lake/v".to_string())
        .await
        .unwrap();
    assert_eq!(volume.volume_type, VolumeType::External);
    assert_eq!(volume.storage_location, "s3://bucket/lake/v");
    rejected(
        uc.credential("other").delete().await,
        400,
        "INVALID_ARGUMENT",
    );
    uc.credential("other")
        .delete()
        .with_force(true)
        .await
        .unwrap();
    rejected(
        uc.external_location("loc2").delete().await,
        400,
        "INVALID_ARGUMENT",
    );
    uc.external_location("loc2")
        .delete()
        .with_force(true)
        .await
        .unwrap();
    assert!(
        uc.list_external_locations()
            .await
            .unwrap()
            .external_locations
            .is_empty()
    );
}
