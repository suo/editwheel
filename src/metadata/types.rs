//! Metadata types for Python wheel METADATA file (PEP 566)

use std::collections::HashMap;

use crate::error::MetadataError;

/// Core metadata per PEP 566/621
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    // Required fields
    pub metadata_version: String,
    pub name: String,
    pub version: String,

    // Optional single-value fields
    pub summary: Option<String>,
    pub description: Option<String>,
    pub description_content_type: Option<String>,
    pub home_page: Option<String>,
    pub download_url: Option<String>,
    pub author: Option<String>,
    pub author_email: Option<String>,
    pub maintainer: Option<String>,
    pub maintainer_email: Option<String>,
    pub license: Option<String>,
    pub keywords: Option<String>,
    pub requires_python: Option<String>,

    // Multi-value fields
    pub classifiers: Vec<String>,
    pub platform: Vec<String>,
    pub requires_dist: Vec<String>,
    pub requires_external: Vec<String>,
    pub project_url: Vec<String>,
    pub provides_extra: Vec<String>,
    pub provides_dist: Vec<String>,
    pub obsoletes_dist: Vec<String>,

    // For preserving unknown headers
    pub extra_headers: HashMap<String, Vec<String>>,
}

impl Metadata {
    /// Parse metadata from RFC822 format content
    pub fn parse(content: &str) -> Result<Self, MetadataError> {
        let mut metadata = Metadata::default();

        // Split into headers and body (separated by blank line)
        let mut in_headers = true;
        let mut current_key: Option<String> = None;
        let mut current_value = String::new();
        let mut body_lines = Vec::new();

        for line in content.lines() {
            if in_headers {
                if line.is_empty() {
                    // End of headers, flush current header
                    if let Some(key) = current_key.take() {
                        metadata.set_field(&key, current_value.trim())?;
                        current_value.clear();
                    }
                    in_headers = false;
                    continue;
                }

                // Check for continuation line (starts with whitespace)
                if line.starts_with(' ') || line.starts_with('\t') {
                    // Continuation of previous header
                    if current_key.is_some() {
                        current_value.push('\n');
                        current_value.push_str(line.trim());
                    }
                    continue;
                }

                // New header line
                if let Some(key) = current_key.take() {
                    metadata.set_field(&key, current_value.trim())?;
                    current_value.clear();
                }

                if let Some((key, value)) = line.split_once(':') {
                    current_key = Some(key.trim().to_string());
                    current_value = value.trim().to_string();
                }
            } else {
                body_lines.push(line);
            }
        }

        // Flush last header if still in headers section
        if let Some(key) = current_key.take() {
            metadata.set_field(&key, current_value.trim())?;
        }

        // Body is the description
        if !body_lines.is_empty() {
            let body = body_lines.join("\n");
            let trimmed = body.trim();
            if !trimmed.is_empty() {
                metadata.description = Some(trimmed.to_string());
            }
        }

        // Validate required fields
        if metadata.name.is_empty() {
            return Err(MetadataError::MissingField("Name".to_string()));
        }
        if metadata.version.is_empty() {
            return Err(MetadataError::MissingField("Version".to_string()));
        }

        Ok(metadata)
    }

    /// Set a metadata field by key
    fn set_field(&mut self, key: &str, value: &str) -> Result<(), MetadataError> {
        match key {
            "Metadata-Version" => self.metadata_version = value.to_string(),
            "Name" => self.name = value.to_string(),
            "Version" => self.version = value.to_string(),
            "Summary" => self.summary = Some(value.to_string()),
            "Description" => self.description = Some(value.to_string()),
            "Description-Content-Type" => self.description_content_type = Some(value.to_string()),
            "Home-page" | "Home-Page" => self.home_page = Some(value.to_string()),
            "Download-URL" => self.download_url = Some(value.to_string()),
            "Author" => self.author = Some(value.to_string()),
            "Author-email" | "Author-Email" => self.author_email = Some(value.to_string()),
            "Maintainer" => self.maintainer = Some(value.to_string()),
            "Maintainer-email" | "Maintainer-Email" => {
                self.maintainer_email = Some(value.to_string())
            }
            "License" => self.license = Some(value.to_string()),
            "Keywords" => self.keywords = Some(value.to_string()),
            "Requires-Python" => self.requires_python = Some(value.to_string()),
            "Classifier" => self.classifiers.push(value.to_string()),
            "Platform" => self.platform.push(value.to_string()),
            "Requires-Dist" => self.requires_dist.push(value.to_string()),
            "Requires-External" => self.requires_external.push(value.to_string()),
            "Project-URL" => self.project_url.push(value.to_string()),
            "Provides-Extra" => self.provides_extra.push(value.to_string()),
            "Provides-Dist" => self.provides_dist.push(value.to_string()),
            "Obsoletes-Dist" => self.obsoletes_dist.push(value.to_string()),
            _ => {
                // Preserve unknown headers
                self.extra_headers
                    .entry(key.to_string())
                    .or_default()
                    .push(value.to_string());
            }
        }
        Ok(())
    }

    /// Serialize metadata back to RFC822 format
    pub fn serialize(&self) -> String {
        use std::fmt::Write;
        let mut output = String::new();

        // Required fields first
        writeln!(output, "Metadata-Version: {}", self.metadata_version).unwrap();
        writeln!(output, "Name: {}", self.name).unwrap();
        writeln!(output, "Version: {}", self.version).unwrap();

        // Optional single-value fields
        if let Some(ref v) = self.summary {
            writeln!(output, "Summary: {}", v).unwrap();
        }
        if let Some(ref v) = self.description_content_type {
            writeln!(output, "Description-Content-Type: {}", v).unwrap();
        }
        if let Some(ref v) = self.home_page {
            writeln!(output, "Home-page: {}", v).unwrap();
        }
        if let Some(ref v) = self.download_url {
            writeln!(output, "Download-URL: {}", v).unwrap();
        }
        if let Some(ref v) = self.author {
            writeln!(output, "Author: {}", v).unwrap();
        }
        if let Some(ref v) = self.author_email {
            writeln!(output, "Author-email: {}", v).unwrap();
        }
        if let Some(ref v) = self.maintainer {
            writeln!(output, "Maintainer: {}", v).unwrap();
        }
        if let Some(ref v) = self.maintainer_email {
            writeln!(output, "Maintainer-email: {}", v).unwrap();
        }
        if let Some(ref v) = self.license {
            writeln!(output, "License: {}", v).unwrap();
        }
        if let Some(ref v) = self.keywords {
            writeln!(output, "Keywords: {}", v).unwrap();
        }
        if let Some(ref v) = self.requires_python {
            writeln!(output, "Requires-Python: {}", v).unwrap();
        }

        // Multi-value fields
        for v in &self.platform {
            writeln!(output, "Platform: {}", v).unwrap();
        }
        for v in &self.classifiers {
            writeln!(output, "Classifier: {}", v).unwrap();
        }
        for v in &self.requires_dist {
            writeln!(output, "Requires-Dist: {}", v).unwrap();
        }
        for v in &self.requires_external {
            writeln!(output, "Requires-External: {}", v).unwrap();
        }
        for v in &self.project_url {
            writeln!(output, "Project-URL: {}", v).unwrap();
        }
        for v in &self.provides_extra {
            writeln!(output, "Provides-Extra: {}", v).unwrap();
        }
        for v in &self.provides_dist {
            writeln!(output, "Provides-Dist: {}", v).unwrap();
        }
        for v in &self.obsoletes_dist {
            writeln!(output, "Obsoletes-Dist: {}", v).unwrap();
        }

        // Extra headers
        for (key, values) in &self.extra_headers {
            for v in values {
                writeln!(output, "{}: {}", key, v).unwrap();
            }
        }

        // Description as body (after blank line)
        if let Some(ref desc) = self.description {
            writeln!(output).unwrap(); // Blank line before body
            write!(output, "{}", desc).unwrap();
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_metadata() {
        let content = r#"Metadata-Version: 2.1
Name: test-package
Version: 1.0.0
Summary: A test package
Author: Test Author

This is the description."#;

        let metadata = Metadata::parse(content).unwrap();
        assert_eq!(metadata.name, "test-package");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.summary, Some("A test package".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(
            metadata.description,
            Some("This is the description.".to_string())
        );
    }

    #[test]
    fn test_parse_multivalue_fields() {
        let content = r#"Metadata-Version: 2.1
Name: test-package
Version: 1.0.0
Classifier: Development Status :: 3 - Alpha
Classifier: Programming Language :: Python :: 3
Requires-Dist: requests>=2.20.0
Requires-Dist: click"#;

        let metadata = Metadata::parse(content).unwrap();
        assert_eq!(metadata.classifiers.len(), 2);
        assert_eq!(metadata.requires_dist.len(), 2);
    }

    #[test]
    fn test_roundtrip() {
        let content = r#"Metadata-Version: 2.1
Name: test-package
Version: 1.0.0
Summary: A test package
Author: Test Author
Classifier: Development Status :: 3 - Alpha
Requires-Dist: requests>=2.20.0

This is the description."#;

        let metadata = Metadata::parse(content).unwrap();
        let serialized = metadata.serialize();
        let reparsed = Metadata::parse(&serialized).unwrap();

        assert_eq!(metadata.name, reparsed.name);
        assert_eq!(metadata.version, reparsed.version);
        assert_eq!(metadata.summary, reparsed.summary);
        assert_eq!(metadata.classifiers, reparsed.classifiers);
    }
}
