use serde::{Deserialize, Serialize};

pub const DOCUMENT_SOURCE_LOCAL_FILE: &str = "local_file";
pub const DOCUMENT_STATUS_IMPORTED: &str = "imported";
pub const MAX_IMPORTED_DOCUMENT_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSummary {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub source_kind: String,
    pub format: String,
    pub mime_type: Option<String>,
    pub stored_path: String,
    pub file_size_bytes: i64,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDocumentsOverview {
    pub project_id: String,
    pub documents: Vec<DocumentSummary>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListProjectDocumentsInput {
    pub project_id: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportDocumentInput {
    pub project_id: String,
    pub file_name: String,
    pub mime_type: Option<String>,
    pub base64_content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDocument {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub source_kind: String,
    pub format: String,
    pub mime_type: Option<String>,
    pub stored_path: String,
    pub file_size_bytes: i64,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}
