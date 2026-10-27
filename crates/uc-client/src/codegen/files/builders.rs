// @generated — do not edit by hand.
#![allow(unused_mut)]
#![allow(unused_imports)]

#[cfg(not(target_arch = "wasm32"))]
type BoxFut<'a, T> = ::futures::future::BoxFuture<'a, T>;
#[cfg(target_arch = "wasm32")]
type BoxFut<'a, T> = ::futures::future::LocalBoxFuture<'a, T>;
use super::client::*;
use crate::codegen::error::Result;
use std::future::IntoFuture;
use uc_client::models::files::v1::*;

/// Builder for directory contents
pub struct ListDirectoryContentsBuilder {
    client: FileClient,
    request: ListDirectoryContentsRequest,
}

impl ListDirectoryContentsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `FileClient`.
    pub(crate) fn new(client: FileClient, directory_path: impl Into<String>) -> Self {
        let request = ListDirectoryContentsRequest {
            directory_path: directory_path.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The maximum number of entries to return. The server returns at most 1000.
    pub fn with_page_size(mut self, page_size: impl Into<Option<i32>>) -> Self {
        self.request.page_size = page_size.into();
        self
    }

    /// Opaque token of the page to return: the `next_page_token` of the page before.
    pub fn with_page_token(mut self, page_token: impl Into<Option<String>>) -> Self {
        self.request.page_token = page_token.into();
        self
    }
}

impl IntoFuture for ListDirectoryContentsBuilder {
    type Output = Result<ListDirectoryContentsResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_directory_contents(&request).await })
    }
}

/// Builder for directory metadata
pub struct GetDirectoryMetadataBuilder {
    client: FileClient,
    request: GetDirectoryMetadataRequest,
}

impl GetDirectoryMetadataBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `FileClient`.
    pub(crate) fn new(client: FileClient, directory_path: impl Into<String>) -> Self {
        let request = GetDirectoryMetadataRequest {
            directory_path: directory_path.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetDirectoryMetadataBuilder {
    type Output = Result<bool>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_directory_metadata(&request).await })
    }
}

/// Builder for directory
pub struct CreateDirectoryBuilder {
    client: FileClient,
    request: CreateDirectoryRequest,
}

impl CreateDirectoryBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `FileClient`.
    pub(crate) fn new(client: FileClient, directory_path: impl Into<String>) -> Self {
        let request = CreateDirectoryRequest {
            directory_path: directory_path.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for CreateDirectoryBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_directory(&request).await })
    }
}

/// Builder for directory
pub struct DeleteDirectoryBuilder {
    client: FileClient,
    request: DeleteDirectoryRequest,
}

impl DeleteDirectoryBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `FileClient`.
    pub(crate) fn new(client: FileClient, directory_path: impl Into<String>) -> Self {
        let request = DeleteDirectoryRequest {
            directory_path: directory_path.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// Also delete the files and directories in it. Without it, a directory that holds anything is
    /// an error. Not in the Databricks Files API.
    pub fn with_recursive(mut self, recursive: impl Into<Option<bool>>) -> Self {
        self.request.recursive = recursive.into();
        self
    }
}

impl IntoFuture for DeleteDirectoryBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_directory(&request).await })
    }
}

/// Builder for file
pub struct DownloadFileBuilder {
    client: FileClient,
    request: DownloadFileRequest,
}

impl DownloadFileBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `FileClient`.
    pub(crate) fn new(client: FileClient, file_path: impl Into<String>) -> Self {
        let request = DownloadFileRequest {
            file_path: file_path.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The bytes to read, as an HTTP `Range` header has them: `bytes=0-1023`, `bytes=1024-` or
    /// `bytes=-1024`. The answer is then status 206, with those bytes.
    pub fn with_range(mut self, range: impl Into<Option<String>>) -> Self {
        self.request.range = range.into();
        self
    }
}

impl IntoFuture for DownloadFileBuilder {
    type Output = Result<DownloadFileResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.download_file(&request).await })
    }
}

/// Builder for file metadata
pub struct GetFileMetadataBuilder {
    client: FileClient,
    request: GetFileMetadataRequest,
}

impl GetFileMetadataBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `FileClient`.
    pub(crate) fn new(client: FileClient, file_path: impl Into<String>) -> Self {
        let request = GetFileMetadataRequest {
            file_path: file_path.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetFileMetadataBuilder {
    type Output = Result<GetFileMetadataResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_file_metadata(&request).await })
    }
}

/// Builder for file
pub struct UploadFileBuilder {
    client: FileClient,
    request: UploadFileRequest,
}

impl UploadFileBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `FileClient`.
    pub(crate) fn new(client: FileClient, file_path: impl Into<String>, contents: Vec<u8>) -> Self {
        let request = UploadFileRequest {
            file_path: file_path.into(),
            contents,
            ..Default::default()
        };
        Self { client, request }
    }

    /// Whether to replace a file that is already there. Without it, an existing file is an error.
    pub fn with_overwrite(mut self, overwrite: impl Into<Option<bool>>) -> Self {
        self.request.overwrite = overwrite.into();
        self
    }
}

impl IntoFuture for UploadFileBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.upload_file(&request).await })
    }
}

/// Builder for file
pub struct DeleteFileBuilder {
    client: FileClient,
    request: DeleteFileRequest,
}

impl DeleteFileBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `FileClient`.
    pub(crate) fn new(client: FileClient, file_path: impl Into<String>) -> Self {
        let request = DeleteFileRequest {
            file_path: file_path.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for DeleteFileBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_file(&request).await })
    }
}
