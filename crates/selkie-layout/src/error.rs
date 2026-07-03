use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum LayoutError {
    #[error("Duplicate node id: {0}")]
    DuplicateNodeId(String),
    #[error("Missing edge endpoint `{endpoint}` for edge `{edge}`")]
    MissingEdgeEndpoint { edge: String, endpoint: String },
    #[error("Invalid parent relationship: {0}")]
    InvalidParent(String),
    #[error("Invalid layout value: {0}")]
    InvalidValue(String),
    #[error("Layout failed: {0}")]
    LayoutFailed(String),
}

pub type LayoutResult<T> = std::result::Result<T, LayoutError>;
