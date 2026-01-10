//! Types for Python wheel WHEEL file (PEP 427)

use std::collections::HashMap;
use std::fmt::Write;

use crate::error::WheelInfoError;

/// Wheel tag representing a compatibility tag (python-abi-platform)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelTag {
    pub python: String,   // e.g., "cp311", "py3"
    pub abi: String,      // e.g., "cp311", "none"
    pub platform: String, // e.g., "linux_x86_64", "manylinux_2_28_x86_64"
}

impl WheelTag {
    /// Parse a tag from string format "python-abi-platform"
    pub fn parse(s: &str) -> Result<Self, WheelInfoError> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return Err(WheelInfoError::InvalidTag(format!(
                "Expected 3 parts (python-abi-platform), got {}: '{}'",
                parts.len(),
                s
            )));
        }
        Ok(Self {
            python: parts[0].to_string(),
            abi: parts[1].to_string(),
            platform: parts[2].to_string(),
        })
    }

    /// Serialize the tag back to string format
    pub fn serialize(&self) -> String {
        format!("{}-{}-{}", self.python, self.abi, self.platform)
    }
}

/// WHEEL file information per PEP 427
#[derive(Debug, Clone, Default)]
pub struct WheelInfo {
    pub wheel_version: String,
    pub generator: Option<String>,
    pub root_is_purelib: bool,
    pub tags: Vec<WheelTag>,
    pub build: Option<String>,
    /// For preserving unknown headers
    pub extra_headers: HashMap<String, Vec<String>>,
}

impl WheelInfo {
    /// Parse WHEEL file content
    pub fn parse(content: &str) -> Result<Self, WheelInfoError> {
        let mut info = WheelInfo::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                info.set_field(key, value)?;
            }
        }

        // Validate required fields
        if info.wheel_version.is_empty() {
            return Err(WheelInfoError::MissingField("Wheel-Version".to_string()));
        }
        if info.tags.is_empty() {
            return Err(WheelInfoError::MissingField("Tag".to_string()));
        }

        Ok(info)
    }

    /// Set a field by key
    fn set_field(&mut self, key: &str, value: &str) -> Result<(), WheelInfoError> {
        match key {
            "Wheel-Version" => self.wheel_version = value.to_string(),
            "Generator" => self.generator = Some(value.to_string()),
            "Root-Is-Purelib" => {
                self.root_is_purelib = value.eq_ignore_ascii_case("true");
            }
            "Tag" => {
                let tag = WheelTag::parse(value)?;
                self.tags.push(tag);
            }
            "Build" => self.build = Some(value.to_string()),
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

    /// Serialize WHEEL info back to file format
    pub fn serialize(&self) -> String {
        let mut output = String::new();

        writeln!(output, "Wheel-Version: {}", self.wheel_version).unwrap();
        if let Some(ref gen) = self.generator {
            writeln!(output, "Generator: {}", gen).unwrap();
        }
        writeln!(
            output,
            "Root-Is-Purelib: {}",
            if self.root_is_purelib { "true" } else { "false" }
        )
        .unwrap();
        for tag in &self.tags {
            writeln!(output, "Tag: {}", tag.serialize()).unwrap();
        }
        if let Some(ref build) = self.build {
            writeln!(output, "Build: {}", build).unwrap();
        }

        // Extra headers
        for (key, values) in &self.extra_headers {
            for v in values {
                writeln!(output, "{}: {}", key, v).unwrap();
            }
        }

        output
    }

    /// Get the primary platform tag (first tag's platform)
    pub fn platform(&self) -> Option<&str> {
        self.tags.first().map(|t| t.platform.as_str())
    }

    /// Set the platform for all tags
    pub fn set_platform(&mut self, platform: &str) {
        for tag in &mut self.tags {
            tag.platform = platform.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wheel_tag() {
        let tag = WheelTag::parse("cp311-cp311-linux_x86_64").unwrap();
        assert_eq!(tag.python, "cp311");
        assert_eq!(tag.abi, "cp311");
        assert_eq!(tag.platform, "linux_x86_64");
    }

    #[test]
    fn test_parse_wheel_info() {
        let content = r#"Wheel-Version: 1.0
Generator: bdist_wheel (0.40.0)
Root-Is-Purelib: false
Tag: cp311-cp311-linux_x86_64
"#;

        let info = WheelInfo::parse(content).unwrap();
        assert_eq!(info.wheel_version, "1.0");
        assert_eq!(info.generator, Some("bdist_wheel (0.40.0)".to_string()));
        assert!(!info.root_is_purelib);
        assert_eq!(info.tags.len(), 1);
        assert_eq!(info.tags[0].platform, "linux_x86_64");
    }

    #[test]
    fn test_parse_multiple_tags() {
        let content = r#"Wheel-Version: 1.0
Generator: test
Root-Is-Purelib: true
Tag: py3-none-any
Tag: py2-none-any
"#;

        let info = WheelInfo::parse(content).unwrap();
        assert_eq!(info.tags.len(), 2);
        assert_eq!(info.tags[0].platform, "any");
        assert_eq!(info.tags[1].python, "py2");
    }

    #[test]
    fn test_set_platform() {
        let content = r#"Wheel-Version: 1.0
Generator: test
Root-Is-Purelib: false
Tag: cp311-cp311-linux_x86_64
"#;

        let mut info = WheelInfo::parse(content).unwrap();
        info.set_platform("manylinux_2_28_x86_64");
        assert_eq!(info.tags[0].platform, "manylinux_2_28_x86_64");
    }

    #[test]
    fn test_roundtrip() {
        let content = r#"Wheel-Version: 1.0
Generator: bdist_wheel (0.40.0)
Root-Is-Purelib: false
Tag: cp311-cp311-linux_x86_64
"#;

        let info = WheelInfo::parse(content).unwrap();
        let serialized = info.serialize();
        let reparsed = WheelInfo::parse(&serialized).unwrap();

        assert_eq!(info.wheel_version, reparsed.wheel_version);
        assert_eq!(info.generator, reparsed.generator);
        assert_eq!(info.root_is_purelib, reparsed.root_is_purelib);
        assert_eq!(info.tags.len(), reparsed.tags.len());
        assert_eq!(info.tags[0], reparsed.tags[0]);
    }
}
