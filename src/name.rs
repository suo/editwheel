//! Name normalization utilities for Python wheels (PEP 427)

/// Normalize a package name for use in dist-info directory names.
///
/// While PEP 503 normalizes to hyphens for PyPI URLs, dist-info directories
/// inside wheels use underscores as separators per PEP 427.
pub fn normalize_dist_info_name(name: &str) -> String {
    // Replace runs of [-_.] with underscore for dist-info dirs
    let mut result = String::with_capacity(name.len());
    let mut in_separator = false;

    for c in name.chars() {
        if c == '-' || c == '_' || c == '.' {
            if !in_separator {
                result.push('_');
                in_separator = true;
            }
            // Skip additional separators
        } else {
            result.push(c);
            in_separator = false;
        }
    }

    result
}

/// Compute the dist-info directory name from package name and version
pub fn dist_info_name(name: &str, version: &str) -> String {
    format!("{}-{}.dist-info", normalize_dist_info_name(name), version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_simple() {
        assert_eq!(normalize_dist_info_name("my-package"), "my_package");
        assert_eq!(normalize_dist_info_name("my_package"), "my_package");
        assert_eq!(normalize_dist_info_name("my.package"), "my_package");
    }

    #[test]
    fn test_normalize_multiple_separators() {
        assert_eq!(normalize_dist_info_name("my--package"), "my_package");
        assert_eq!(normalize_dist_info_name("my.-_package"), "my_package");
    }

    #[test]
    fn test_dist_info_name() {
        assert_eq!(
            dist_info_name("my-package", "1.0.0"),
            "my_package-1.0.0.dist-info"
        );
    }
}
