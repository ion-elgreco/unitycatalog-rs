//! The credentials, from "Manage" in the sidebar: each one's name,
//! identity, owner and creation, and "Create credential". The server lists
//! the ones the user may see: all of them to admins, else their own and the
//! ones they may bind external locations to. Upstream's console has no
//! such page.

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_client::models::credentials::v1::{
    AwsIamRoleRequest, AzureManagedIdentity, AzureServicePrincipalRequest, CredentialInfo,
    CredentialPurpose, GcpServiceAccount, GcpServiceAccountKeyRequest,
};

use crate::api;
use crate::components::icons::Plus;
use crate::components::{Empty, Modal, Select, TextField, use_notify};
use crate::layout::{Column, ListLayout};
use crate::load::{Key, use_cache, use_load};
use crate::util::{Section, format_timestamp, or_dash};

/// The kinds of identity that a credential holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityKind {
    AwsIamRole,
    AzureServicePrincipal,
    AzureManagedIdentity,
    GcpServiceAccount,
    GcpServiceAccountKey,
}

impl IdentityKind {
    const ALL: [IdentityKind; 5] = [
        Self::AwsIamRole,
        Self::AzureServicePrincipal,
        Self::AzureManagedIdentity,
        Self::GcpServiceAccount,
        Self::GcpServiceAccountKey,
    ];

    pub fn text(self) -> &'static str {
        match self {
            Self::AwsIamRole => "AWS IAM role",
            Self::AzureServicePrincipal => "Azure service principal",
            Self::AzureManagedIdentity => "Azure managed identity",
            Self::GcpServiceAccount => "GCP service account",
            Self::GcpServiceAccountKey => "GCP service account key",
        }
    }

    /// The kind of a credential's identity.
    pub fn of(c: &CredentialInfo) -> Option<Self> {
        if c.aws_iam_role.is_set() {
            Some(Self::AwsIamRole)
        } else if c.azure_service_principal.is_set() {
            Some(Self::AzureServicePrincipal)
        } else if c.azure_managed_identity.is_set() {
            Some(Self::AzureManagedIdentity)
        } else if c.gcp_service_account.is_set() {
            Some(Self::GcpServiceAccount)
        } else if c.gcp_service_account_key.is_set() {
            Some(Self::GcpServiceAccountKey)
        } else {
            None
        }
    }
}

/// What names a credential's identity: the ARN of its AWS IAM role, the
/// application ID of its service principal, the resource ID of its managed
/// identity, or the email of its GCP service account. A managed identity or
/// service account without one is the server's own.
pub fn identity_name(c: &CredentialInfo) -> Option<String> {
    let name = match IdentityKind::of(c)? {
        IdentityKind::AwsIamRole => c.aws_iam_role.as_option()?.role_arn.clone(),
        IdentityKind::AzureServicePrincipal => c
            .azure_service_principal
            .as_option()?
            .application_id
            .clone(),
        IdentityKind::AzureManagedIdentity => c
            .azure_managed_identity
            .as_option()?
            .managed_identity_id
            .clone()?,
        IdentityKind::GcpServiceAccount => c.gcp_service_account.as_option()?.email.clone()?,
        IdentityKind::GcpServiceAccountKey => c.gcp_service_account_key.as_option()?.email.clone(),
    };
    (!name.is_empty()).then_some(name)
}

/// The server's own identity: a managed identity without a resource ID, or
/// a GCP service account without an email.
fn servers_own(c: &CredentialInfo) -> bool {
    (c.azure_managed_identity.is_set() || c.gcp_service_account.is_set())
        && identity_name(c).is_none()
}

/// A credential's identity, as a request sends it.
pub enum Identity {
    AwsIamRole(AwsIamRoleRequest),
    AzureServicePrincipal(AzureServicePrincipalRequest),
    AzureManagedIdentity(AzureManagedIdentity),
    GcpServiceAccount(GcpServiceAccount),
    GcpServiceAccountKey(GcpServiceAccountKeyRequest),
}

/// The identity fields of a dialog.
#[derive(Clone, Copy)]
pub struct IdentityForm {
    pub kind: RwSignal<IdentityKind>,
    role_arn: RwSignal<String>,
    directory_id: RwSignal<String>,
    application_id: RwSignal<String>,
    client_secret: RwSignal<String>,
    managed_identity_id: RwSignal<String>,
    email: RwSignal<String>,
    json_key: RwSignal<String>,
}

impl Default for IdentityForm {
    fn default() -> Self {
        Self {
            kind: RwSignal::new(IdentityKind::AwsIamRole),
            role_arn: RwSignal::new(String::new()),
            directory_id: RwSignal::new(String::new()),
            application_id: RwSignal::new(String::new()),
            client_secret: RwSignal::new(String::new()),
            managed_identity_id: RwSignal::new(String::new()),
            email: RwSignal::new(String::new()),
            json_key: RwSignal::new(String::new()),
        }
    }
}

impl IdentityForm {
    /// The identity of `c`. The client secret and the JSON key stay empty:
    /// the server does not return them.
    pub fn fill(&self, c: &CredentialInfo) {
        self.clear();
        self.kind
            .set(IdentityKind::of(c).unwrap_or(IdentityKind::AwsIamRole));
        if let Some(role) = c.aws_iam_role.as_option() {
            self.role_arn.set(role.role_arn.clone());
        }
        if let Some(principal) = c.azure_service_principal.as_option() {
            self.directory_id.set(principal.directory_id.clone());
            self.application_id.set(principal.application_id.clone());
        }
        if let Some(identity) = c.azure_managed_identity.as_option() {
            self.managed_identity_id
                .set(identity.managed_identity_id.clone().unwrap_or_default());
        }
        if let Some(account) = c.gcp_service_account.as_option() {
            self.email.set(account.email.clone().unwrap_or_default());
        }
    }

    pub fn clear(&self) {
        self.kind.set(IdentityKind::AwsIamRole);
        for text in [
            self.role_arn,
            self.directory_id,
            self.application_id,
            self.client_secret,
            self.managed_identity_id,
            self.email,
            self.json_key,
        ] {
            text.set(String::new());
        }
    }

    fn text(field: RwSignal<String>) -> String {
        field.get_untracked().trim().to_string()
    }

    /// The identity of the fields, or what is missing.
    pub fn request(&self) -> Result<Identity, &'static str> {
        let required = |field: RwSignal<String>, missing: &'static str| {
            let text = Self::text(field);
            if text.is_empty() {
                Err(missing)
            } else {
                Ok(text)
            }
        };
        Ok(match self.kind.get_untracked() {
            IdentityKind::AwsIamRole => Identity::AwsIamRole(AwsIamRoleRequest {
                role_arn: required(self.role_arn, "AWS IAM role ARN is required")?,
                ..Default::default()
            }),
            IdentityKind::AzureServicePrincipal => {
                Identity::AzureServicePrincipal(AzureServicePrincipalRequest {
                    directory_id: required(self.directory_id, "Directory ID is required")?,
                    application_id: required(self.application_id, "Application ID is required")?,
                    client_secret: required(self.client_secret, "Client secret is required")?,
                    ..Default::default()
                })
            }
            IdentityKind::AzureManagedIdentity => {
                let id = Self::text(self.managed_identity_id);
                Identity::AzureManagedIdentity(AzureManagedIdentity {
                    managed_identity_id: (!id.is_empty()).then_some(id),
                    ..Default::default()
                })
            }
            IdentityKind::GcpServiceAccount => {
                let email = Self::text(self.email);
                Identity::GcpServiceAccount(GcpServiceAccount {
                    email: (!email.is_empty()).then_some(email),
                    ..Default::default()
                })
            }
            IdentityKind::GcpServiceAccountKey => {
                Identity::GcpServiceAccountKey(GcpServiceAccountKeyRequest {
                    json_key: required(self.json_key, "JSON key is required")?,
                    ..Default::default()
                })
            }
        })
    }

    /// The identity that an edit of `current` sends: none when the fields
    /// hold `current`'s identity. A service principal changes only with its
    /// client secret, a service account key only with a new key.
    pub fn change(&self, current: &CredentialInfo) -> Result<Option<Identity>, &'static str> {
        let kind = self.kind.get_untracked();
        let unchanged = IdentityKind::of(current) == Some(kind)
            && match kind {
                IdentityKind::AwsIamRole => current
                    .aws_iam_role
                    .as_option()
                    .is_some_and(|r| r.role_arn == Self::text(self.role_arn)),
                IdentityKind::AzureServicePrincipal => current
                    .azure_service_principal
                    .as_option()
                    .is_some_and(|p| {
                        p.directory_id == Self::text(self.directory_id)
                            && p.application_id == Self::text(self.application_id)
                            && Self::text(self.client_secret).is_empty()
                    }),
                IdentityKind::AzureManagedIdentity => {
                    current.azure_managed_identity.as_option().is_some_and(|i| {
                        i.managed_identity_id.clone().unwrap_or_default()
                            == Self::text(self.managed_identity_id)
                    })
                }
                IdentityKind::GcpServiceAccount => current
                    .gcp_service_account
                    .as_option()
                    .is_some_and(|a| a.email.clone().unwrap_or_default() == Self::text(self.email)),
                IdentityKind::GcpServiceAccountKey => Self::text(self.json_key).is_empty(),
            };
        match unchanged {
            true => Ok(None),
            false => self.request().map(Some),
        }
    }
}

/// The type of identity and its fields.
#[component]
pub fn IdentityFields(form: IdentityForm, id: &'static str) -> impl IntoView {
    view! {
        <Select
            id=format!("{id}-type")
            label="Type"
            choices=&IdentityKind::ALL
            value=form.kind
            name=|k: &IdentityKind| k.text().to_string()
        />
        {move || match form.kind.get() {
            IdentityKind::AwsIamRole => {
                view! {
                    <TextField
                        id=format!("{id}-role")
                        label="AWS IAM role ARN"
                        value=form.role_arn
                        placeholder="arn:aws:iam::123456789012:role/unity-catalog"
                    />
                    <p class="uc-hint">"The role that the server assumes to reach the storage."</p>
                }
                    .into_any()
            }
            IdentityKind::AzureServicePrincipal => {
                view! {
                    <TextField
                        id=format!("{id}-directory")
                        label="Directory (tenant) ID"
                        value=form.directory_id
                    />
                    <TextField
                        id=format!("{id}-application")
                        label="Application (client) ID"
                        value=form.application_id
                    />
                    <TextField
                        id=format!("{id}-secret")
                        label="Client secret"
                        value=form.client_secret
                        autocomplete="new-password"
                        secret=true
                    />
                    <p class="uc-hint">
                        "The app that the server signs in as. The server keeps the secret and never shows it."
                    </p>
                }
                    .into_any()
            }
            IdentityKind::AzureManagedIdentity => {
                view! {
                    <TextField
                        id=format!("{id}-managed")
                        label="Managed identity resource ID"
                        value=form.managed_identity_id
                        placeholder="/subscriptions/…/userAssignedIdentities/unity-catalog"
                    />
                    <p class="uc-hint">
                        "Empty: the server's own Azure identity. Else a user-assigned identity of the server's VM."
                    </p>
                }
                    .into_any()
            }
            IdentityKind::GcpServiceAccount => {
                view! {
                    <TextField
                        id=format!("{id}-email")
                        label="Service account email"
                        value=form.email
                        placeholder="unity-catalog@my-project.iam.gserviceaccount.com"
                    />
                    <p class="uc-hint">
                        "Empty: the server's own GCP identity. Else a service account that the server impersonates: the server's identity needs the role Service Account Token Creator on it."
                    </p>
                }
                    .into_any()
            }
            IdentityKind::GcpServiceAccountKey => {
                view! {
                    <TextField
                        id=format!("{id}-key")
                        label="JSON key"
                        value=form.json_key
                        autocomplete="off"
                        secret=true
                    />
                    <p class="uc-hint">
                        "Paste the service account's JSON key file. The server keeps the key and never shows it."
                    </p>
                }
                    .into_any()
            }
        }}
    }
}

#[component]
pub fn Credentials() -> impl IntoView {
    let credentials = use_load(Key::Credentials, api::credentials);
    let create_open = RwSignal::new(false);
    let columns = vec![
        Column {
            title: "Name",
            width: "24%",
            render: Box::new(|c: &CredentialInfo| {
                let name = c.name.clone();
                view! {
                    <a class="uc-btn uc-btn--link" href=Section::Credentials.item_href(&name)>
                        {name.clone()}
                    </a>
                }
                .into_any()
            }),
            search_key: Some(Box::new(|c: &CredentialInfo| c.name.clone())),
        },
        Column {
            title: "Identity",
            width: "36%",
            render: Box::new(|c: &CredentialInfo| {
                let kind = IdentityKind::of(c).map(IdentityKind::text);
                view! {
                    <span class="uc-muted">{kind}</span>
                    " "
                    {match servers_own(c) {
                        true => view! { <span class="uc-muted">"(server's own)"</span> }.into_any(),
                        false => view! { <code>{or_dash(identity_name(c))}</code> }.into_any(),
                    }}
                }
                .into_any()
            }),
            search_key: Some(Box::new(|c: &CredentialInfo| {
                let kind = IdentityKind::of(c).map_or("", IdentityKind::text);
                let name = match servers_own(c) {
                    true => "server's own".to_string(),
                    false => identity_name(c).unwrap_or_default(),
                };
                format!("{kind} {name}")
            })),
        },
        Column {
            title: "Owner",
            width: "20%",
            render: Box::new(|c: &CredentialInfo| {
                view! { <span class="uc-muted">{or_dash(c.owner.clone())}</span> }.into_any()
            }),
            search_key: Some(Box::new(|c: &CredentialInfo| {
                c.owner.clone().unwrap_or_default()
            })),
        },
        Column {
            title: "Created",
            width: "20%",
            render: Box::new(|c: &CredentialInfo| {
                view! { <span class="uc-muted uc-nowrap">{format_timestamp(c.created_at)}</span> }
                    .into_any()
            }),
            search_key: None,
        },
    ];
    view! {
        <div class="uc-page">
            <div class="uc-between">
                <h1 class="uc-page__title">"Credentials"</h1>
                <button type="button" class="uc-btn uc-btn--sm" on:click=move |_| create_open.set(true)>
                    <Plus/>
                    "Create credential"
                </button>
            </div>
            <p class="uc-muted">
                "A storage credential holds the cloud identity that the server uses to reach storage: an AWS IAM role that it assumes, an Azure service principal or managed identity that it signs in as, or a GCP service account or service account key. Each external location is bound to one."
            </p>
            <ListLayout
                data=credentials
                columns
                empty=Empty::new(
                    "No credentials.",
                    "Click Create credential to make one, then bind external locations to it.",
                )
            />
            <CreateCredential open=create_open/>
        </div>
    }
}

#[component]
fn CreateCredential(open: RwSignal<bool>) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let identity = IdentityForm::default();
    let comment = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let close = move || {
        open.set(false);
        name.set(String::new());
        identity.clear();
        comment.set(String::new());
    };
    let on_ok = move || {
        let new_name = name.get_untracked().trim().to_string();
        if new_name.is_empty() {
            notify.error("Name is required");
            return;
        }
        let request = match identity.request() {
            Ok(request) => request,
            Err(missing) => {
                notify.error(missing);
                return;
            }
        };
        let text = comment.get_untracked();
        let create = api::client()
            .create_credential(new_name)
            .with_comment((!text.is_empty()).then_some(text))
            .with_purpose(CredentialPurpose::Storage);
        let create = match request {
            Identity::AwsIamRole(role) => create.with_aws_iam_role(role),
            Identity::AzureServicePrincipal(principal) => {
                create.with_azure_service_principal(principal)
            }
            Identity::AzureManagedIdentity(managed) => create.with_azure_managed_identity(managed),
            Identity::GcpServiceAccount(account) => create.with_gcp_service_account(account),
            Identity::GcpServiceAccountKey(key) => create.with_gcp_service_account_key(key),
        };
        busy.set(true);
        spawn_local(async move {
            match create.await {
                Ok(_) => {
                    notify.success("Create credential complete");
                    cache.changed(Key::Credentials);
                    // Only when the user is still on the page.
                    if busy.try_get_untracked().is_some() {
                        close();
                    }
                }
                Err(e) => notify.failed(&e, "Create credential failed"),
            }
            busy.set(false);
        });
    };
    view! {
        <Modal
            open
            title="Create credential"
            ok_text="Create"
            busy
            on_cancel=move || close()
            on_ok
        >
            <TextField id="uc-credential-name" label="Name" value=name required=true/>
            <IdentityFields form=identity id="uc-credential"/>
            <TextField id="uc-credential-comment" label="Comment" value=comment/>
        </Modal>
    }
}
