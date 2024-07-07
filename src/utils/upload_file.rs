use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct UploadedFile {
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub s3_key: String,
    pub s3_url: String,
}

impl UploadedFile {
    pub fn new(
        filename: impl Into<String>,
        content_type: impl Into<String>,
        size: i64,
        s3_key: impl Into<String>,
        s3_url: impl Into<String>,
    ) -> Self {
        Self {
            filename: filename.into(),
            content_type: content_type.into(),
            size,
            s3_key: s3_key.into(),
            s3_url: s3_url.into(),
        }
    }
}
