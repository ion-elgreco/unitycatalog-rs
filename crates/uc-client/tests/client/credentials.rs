//! Credentials and external locations, and the deletes that need `force`.

use futures::TryStreamExt;

use uc_client::models::credentials::v1::{
    AwsIamRoleRequest, AzureManagedIdentity, AzureServicePrincipalRequest, CredentialPurpose,
    GcpServiceAccount, GcpServiceAccountKeyRequest,
};
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
        .create_credential("cred")
        .with_aws_iam_role(role(ROLE))
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
    uc.create_credential("service")
        .with_aws_iam_role(role(ROLE))
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

/// Azure credentials: a service principal, whose secret no answer holds,
/// and a managed identity. A location takes only a credential that reaches
/// its storage.
#[tokio::test]
async fn azure_credentials() {
    let uc = client().await;
    let principal = AzureServicePrincipalRequest {
        directory_id: "tenant".into(),
        application_id: "app".into(),
        client_secret: "secret".into(),
        ..Default::default()
    };
    let credential = uc
        .create_credential("sp")
        .with_azure_service_principal(principal)
        .await
        .unwrap();
    let sp = credential.azure_service_principal.as_option().unwrap();
    assert_eq!(
        (sp.directory_id.as_str(), sp.application_id.as_str()),
        ("tenant", "app")
    );
    assert!(!credential.aws_iam_role.is_set());
    let url = "abfss://c@acct.dfs.core.windows.net/lake";
    uc.create_external_location("lake", url, "sp")
        .await
        .unwrap();
    rejected(
        uc.create_external_location("s3", "s3://bucket/lake", "sp")
            .await,
        400,
        "INVALID_ARGUMENT",
    );

    let id = "/subscriptions/s/resourceGroups/g/providers/Microsoft.ManagedIdentity/userAssignedIdentities/uc";
    let updated = uc
        .credential("sp")
        .update()
        .with_azure_managed_identity(AzureManagedIdentity {
            managed_identity_id: Some(id.into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(!updated.azure_service_principal.is_set());
    let managed = updated.azure_managed_identity.as_option().unwrap();
    assert_eq!(managed.managed_identity_id.as_deref(), Some(id));
    // The location's storage needs an Azure identity.
    rejected(
        uc.credential("sp")
            .update()
            .with_aws_iam_role(role(ROLE))
            .await,
        400,
        "INVALID_ARGUMENT",
    );
}

#[tokio::test]
async fn gcp_credentials() {
    let uc = client().await;
    let json_key = serde_json::json!({
        "type": "service_account",
        "client_email": "uc@p.iam.gserviceaccount.com",
        "private_key_id": "kid",
        "private_key": "PRIVATE",
    });
    let credential = uc
        .create_credential("key")
        .with_gcp_service_account_key(GcpServiceAccountKeyRequest {
            json_key: json_key.to_string(),
            ..Default::default()
        })
        .await
        .unwrap();
    let key = credential.gcp_service_account_key.as_option().unwrap();
    assert_eq!(
        (key.email.as_str(), key.private_key_id.as_str()),
        ("uc@p.iam.gserviceaccount.com", "kid")
    );
    uc.create_external_location("lake", "gs://bucket/lake", "key")
        .await
        .unwrap();
    rejected(
        uc.create_external_location("s3", "s3://bucket/lake", "key")
            .await,
        400,
        "INVALID_ARGUMENT",
    );

    let email = "data@p.iam.gserviceaccount.com";
    let updated = uc
        .credential("key")
        .update()
        .with_gcp_service_account(GcpServiceAccount {
            email: Some(email.into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(!updated.gcp_service_account_key.is_set());
    let account = updated.gcp_service_account.as_option().unwrap();
    assert_eq!(account.email.as_deref(), Some(email));
}

/// A credential that a location uses, and a location that a volume uses,
/// go only with `force`.
#[tokio::test]
async fn external_locations() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let credential = uc
        .create_credential("cred")
        .with_aws_iam_role(role(ROLE))
        .await
        .unwrap();
    uc.create_credential("other")
        .with_aws_iam_role(role(ROLE))
        .await
        .unwrap();
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
