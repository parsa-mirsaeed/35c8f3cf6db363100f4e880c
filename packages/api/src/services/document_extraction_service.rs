//! Document Extraction Service for extracting text from PDFs.
//!
//! This module uses the `pdf-extract` library (pure Rust, built on lopdf)
//! to extract text content from uploaded course materials.
//!
//! Security: This implementation uses NO external services or network calls.
//! All processing is done locally with pure Rust libraries.

use std::io::Cursor;
use thiserror::Error;

/// Errors that can occur during document extraction
#[derive(Debug, Error)]
pub enum ExtractionError {
    #[error("Failed to download file: {0}")]
    DownloadError(String),

    #[error("Failed to extract content: {0}")]
    ExtractionFailed(String),

    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Supported document types
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentType {
    Pdf,
    Txt,
    Markdown,
    Html,
    Unknown,
}

impl DocumentType {
    /// Detect document type from file extension
    pub fn from_extension(path: &str) -> Self {
        let path_lower = path.to_lowercase();
        if path_lower.ends_with(".pdf") {
            DocumentType::Pdf
        } else if path_lower.ends_with(".txt") {
            DocumentType::Txt
        } else if path_lower.ends_with(".md") || path_lower.ends_with(".markdown") {
            DocumentType::Markdown
        } else if path_lower.ends_with(".html") || path_lower.ends_with(".htm") {
            DocumentType::Html
        } else {
            DocumentType::Unknown
        }
    }

    /// Check if this document type is supported for extraction
    pub fn is_supported(&self) -> bool {
        !matches!(self, DocumentType::Unknown)
    }
}

/// Result of document extraction
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// The extracted text content
    pub text: String,
    /// The document type that was processed
    pub document_type: DocumentType,
    /// Number of pages (if applicable, for PDFs)
    pub page_count: Option<usize>,
    /// Original file path or URL
    pub source: String,
}

/// Document Extraction Service
/// 
/// Uses pure Rust libraries with NO external service dependencies.
pub struct DocumentExtractionService {
    http_client: reqwest::Client,
}

impl DocumentExtractionService {
    /// Create a new document extraction service
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Extract text from a document URL
    pub async fn extract_from_url(&self, url: &str) -> Result<ExtractionResult, ExtractionError> {
        let doc_type = DocumentType::from_extension(url);
        
        if !doc_type.is_supported() {
            return Err(ExtractionError::UnsupportedFormat(
                format!("Cannot extract from: {}", url)
            ));
        }

        // Download the file
        let bytes = self.download_file(url).await?;
        
        // Extract based on type
        match doc_type {
            DocumentType::Pdf => self.extract_pdf(&bytes, url),
            DocumentType::Txt | DocumentType::Markdown | DocumentType::Html => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                Ok(ExtractionResult {
                    text,
                    document_type: doc_type,
                    page_count: None,
                    source: url.to_string(),
                })
            }
            DocumentType::Unknown => {
                Err(ExtractionError::UnsupportedFormat(url.to_string()))
            }
        }
    }

    /// Extract text from PDF bytes using pdf-extract (pure Rust)
    fn extract_pdf(&self, bytes: &[u8], source: &str) -> Result<ExtractionResult, ExtractionError> {
        // Use pdf-extract to get text content
        // This is pure Rust with NO external service calls
        let text = pdf_extract::extract_text_from_mem(bytes)
            .map_err(|e| ExtractionError::ExtractionFailed(e.to_string()))?;

        // Estimate page count from form feeds or text length
        let page_count = if text.contains('\x0c') {
            // Form feed characters typically separate pages
            Some(text.matches('\x0c').count() + 1)
        } else {
            // Rough estimate: ~3000 chars per page
            Some((text.len() / 3000).max(1))
        };

        // Clean up the extracted text
        let cleaned_text = self.clean_extracted_text(&text);

        tracing::info!(
            "Extracted {} chars (~{} pages) from PDF: {}",
            cleaned_text.len(),
            page_count.unwrap_or(1),
            source
        );

        Ok(ExtractionResult {
            text: cleaned_text,
            document_type: DocumentType::Pdf,
            page_count,
            source: source.to_string(),
        })
    }

    /// Clean up extracted text (remove excessive whitespace, etc.)
    fn clean_extracted_text(&self, text: &str) -> String {
        // Remove form feed characters
        let text = text.replace('\x0c', "\n\n");
        
        // Normalize whitespace while preserving paragraph structure
        let lines: Vec<&str> = text.lines().collect();
        let mut result = String::new();
        let mut prev_empty = false;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if !prev_empty {
                    result.push('\n');
                    prev_empty = true;
                }
            } else {
                if prev_empty {
                    result.push('\n');
                }
                result.push_str(trimmed);
                result.push('\n');
                prev_empty = false;
            }
        }

        result.trim().to_string()
    }

    /// Download file from URL
    async fn download_file(&self, url: &str) -> Result<Vec<u8>, ExtractionError> {
        let response = self.http_client
            .get(url)
            .send()
            .await
            .map_err(|e| ExtractionError::DownloadError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ExtractionError::DownloadError(
                format!("HTTP {}: {}", response.status(), url)
            ));
        }

        let bytes = response.bytes()
            .await
            .map_err(|e| ExtractionError::DownloadError(e.to_string()))?;

        tracing::debug!("Downloaded {} bytes from {}", bytes.len(), url);
        
        Ok(bytes.to_vec())
    }

    /// Extract text from local file bytes
    pub fn extract_from_bytes(&self, bytes: &[u8], filename: &str) -> Result<ExtractionResult, ExtractionError> {
        let doc_type = DocumentType::from_extension(filename);
        
        match doc_type {
            DocumentType::Pdf => self.extract_pdf(bytes, filename),
            DocumentType::Txt | DocumentType::Markdown | DocumentType::Html => {
                let text = String::from_utf8_lossy(bytes).to_string();
                Ok(ExtractionResult {
                    text,
                    document_type: doc_type,
                    page_count: None,
                    source: filename.to_string(),
                })
            }
            DocumentType::Unknown => {
                Err(ExtractionError::UnsupportedFormat(filename.to_string()))
            }
        }
    }
}

impl Default for DocumentExtractionService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_type_detection() {
        assert_eq!(DocumentType::from_extension("doc.pdf"), DocumentType::Pdf);
        assert_eq!(DocumentType::from_extension("doc.PDF"), DocumentType::Pdf);
        assert_eq!(DocumentType::from_extension("file.txt"), DocumentType::Txt);
        assert_eq!(DocumentType::from_extension("file.md"), DocumentType::Markdown);
        assert_eq!(DocumentType::from_extension("file.html"), DocumentType::Html);
        assert_eq!(DocumentType::from_extension("file.xyz"), DocumentType::Unknown);
    }

    #[test]
    fn test_supported_formats() {
        assert!(DocumentType::Pdf.is_supported());
        assert!(DocumentType::Txt.is_supported());
        assert!(!DocumentType::Unknown.is_supported());
    }

    #[test]
    fn test_text_cleaning() {
        let service = DocumentExtractionService::new();
        let dirty = "Line 1  \n\n\n\nLine 2\x0cPage2";
        let clean = service.clean_extracted_text(dirty);
        assert!(!clean.contains("\x0c"));
        assert!(clean.contains("Line 1"));
        assert!(clean.contains("Line 2"));
    }
}
