//! AWS identities: a signed `sts:GetCallerIdentity` request as the bearer
//! value, checked by [`uc_auth::AwsResolver`] (see [`uc_auth::aws`]).
//!
//! A block of `[[auth.issuers]]` with `kind = "aws"` is one
//! [`uc_auth::AwsProvider`]: its `issuer` is the STS endpoint, its
//! `audiences` are the server ids that the requests sign in
//! [`SERVER_ID_HEADER`], and its `accounts` are the accounts whose
//! identities it takes. The caller is the user of the ARN, an assumed role
//! by its role, named as the block says (`crate::authn::idp::resolve`).

/// The start of an AWS bearer value.
pub const PREFIX: &str = "uc-aws-v1.";
/// The signed header with the server id, one of the block's `audiences`.
pub const SERVER_ID_HEADER: &str = "x-uc-server-id";
