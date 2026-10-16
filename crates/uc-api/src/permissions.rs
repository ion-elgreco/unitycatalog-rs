use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A privilege. The upstream spec names privileges with spaces, such as
/// `"CREATE CATALOG"`, not `"CREATE_CATALOG"`, so each variant has an
/// explicit `serde(rename)`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
pub enum Privilege {
    #[serde(rename = "CREATE CATALOG")]
    CreateCatalog,
    #[serde(rename = "USE CATALOG")]
    UseCatalog,
    #[serde(rename = "CREATE SCHEMA")]
    CreateSchema,
    #[serde(rename = "USE SCHEMA")]
    UseSchema,
    #[serde(rename = "CREATE TABLE")]
    CreateTable,
    #[serde(rename = "SELECT")]
    Select,
    #[serde(rename = "MODIFY")]
    Modify,
    #[serde(rename = "CREATE FUNCTION")]
    CreateFunction,
    #[serde(rename = "EXECUTE")]
    Execute,
    #[serde(rename = "CREATE VOLUME")]
    CreateVolume,
    #[serde(rename = "READ VOLUME")]
    ReadVolume,
    #[serde(rename = "CREATE MODEL")]
    CreateModel,
    #[serde(rename = "CREATE EXTERNAL LOCATION")]
    CreateExternalLocation,
    #[serde(rename = "READ FILES")]
    ReadFiles,
    #[serde(rename = "WRITE FILES")]
    WriteFiles,
    #[serde(rename = "CREATE EXTERNAL TABLE")]
    CreateExternalTable,
    #[serde(rename = "CREATE EXTERNAL VOLUME")]
    CreateExternalVolume,
    #[serde(rename = "CREATE MANAGED STORAGE")]
    CreateManagedStorage,
    #[serde(rename = "CREATE STORAGE CREDENTIAL")]
    CreateStorageCredential,
}

impl Privilege {
    /// Every privilege variant, in declaration order. The Cedar engine builds
    /// its action UID table from it, so a new variant must be added here.
    pub const ALL: &'static [Privilege] = &[
        Privilege::CreateCatalog,
        Privilege::UseCatalog,
        Privilege::CreateSchema,
        Privilege::UseSchema,
        Privilege::CreateTable,
        Privilege::Select,
        Privilege::Modify,
        Privilege::CreateFunction,
        Privilege::Execute,
        Privilege::CreateVolume,
        Privilege::ReadVolume,
        Privilege::CreateModel,
        Privilege::CreateExternalLocation,
        Privilege::ReadFiles,
        Privilege::WriteFiles,
        Privilege::CreateExternalTable,
        Privilege::CreateExternalVolume,
        Privilege::CreateManagedStorage,
        Privilege::CreateStorageCredential,
    ];
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrivilegeAssignment {
    pub principal: String,
    pub privileges: Vec<Privilege>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PermissionsList {
    pub privilege_assignments: Vec<PrivilegeAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PermissionsChange {
    pub principal: String,
    /// Upstream's server takes a missing list as empty.
    #[serde(default)]
    pub add: Vec<Privilege>,
    #[serde(default)]
    pub remove: Vec<Privilege>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdatePermissions {
    pub changes: Vec<PermissionsChange>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::assert_wire_names;
    use serde_json::json;

    #[test]
    fn privileges_match_upstream() {
        // Upstream `Privilege`, in its order.
        let names = [
            "CREATE CATALOG",
            "USE CATALOG",
            "CREATE SCHEMA",
            "USE SCHEMA",
            "CREATE TABLE",
            "SELECT",
            "MODIFY",
            "CREATE FUNCTION",
            "EXECUTE",
            "CREATE VOLUME",
            "READ VOLUME",
            "CREATE MODEL",
            "CREATE EXTERNAL LOCATION",
            "READ FILES",
            "WRITE FILES",
            "CREATE EXTERNAL TABLE",
            "CREATE EXTERNAL VOLUME",
            "CREATE MANAGED STORAGE",
            "CREATE STORAGE CREDENTIAL",
        ];
        assert_eq!(Privilege::ALL.len(), names.len());
        let cases: Vec<_> = Privilege::ALL.iter().copied().zip(names).collect();
        assert_wire_names(&cases);
    }

    #[test]
    fn all_is_in_declaration_order_without_repeats() {
        // The derived `Ord` follows the declaration order.
        assert!(Privilege::ALL.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn missing_change_lists_are_empty() {
        let change: PermissionsChange =
            serde_json::from_value(json!({ "principal": "a@example.com", "add": ["SELECT"] }))
                .unwrap();
        assert_eq!(change.add, [Privilege::Select]);
        assert!(change.remove.is_empty());
    }
}
