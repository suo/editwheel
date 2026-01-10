//! Python bindings for editwheel using PyO3

use pyo3::exceptions::PyFileNotFoundError;
use pyo3::exceptions::PyIOError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::WheelEditor;
use crate::WheelError;
use crate::normalize_dist_info_name as rust_normalize_dist_info_name;

/// Convert WheelError to PyErr
impl From<WheelError> for PyErr {
    fn from(err: WheelError) -> PyErr {
        match &err {
            WheelError::Io(io_err) => {
                if io_err.kind() == std::io::ErrorKind::NotFound {
                    PyFileNotFoundError::new_err(err.to_string())
                } else {
                    PyIOError::new_err(err.to_string())
                }
            }
            WheelError::InvalidWheel(_) => PyValueError::new_err(err.to_string()),
            WheelError::Metadata(_) => PyValueError::new_err(err.to_string()),
            WheelError::Record(_) => PyValueError::new_err(err.to_string()),
            WheelError::Zip(_) => PyIOError::new_err(err.to_string()),
        }
    }
}

/// A class to edit Python wheel metadata and repack the wheel.
///
/// This is a high-performance Rust implementation that achieves constant-time
/// editing regardless of wheel size by copying unchanged files as raw
/// compressed bytes.
#[pyclass(name = "WheelEditor")]
pub struct PyWheelEditor {
    inner: WheelEditor,
}

#[pymethods]
impl PyWheelEditor {
    /// Initialize the WheelEditor with a path to a wheel file.
    ///
    /// Args:
    ///     wheel_path: Path to the wheel file to edit
    ///
    /// Raises:
    ///     FileNotFoundError: If wheel file does not exist
    ///     ValueError: If file is not a valid wheel
    #[new]
    fn new(wheel_path: &str) -> PyResult<Self> {
        // Check file extension
        if !wheel_path.ends_with(".whl") {
            return Err(PyValueError::new_err("File does not have .whl extension"));
        }

        let editor = WheelEditor::open(wheel_path)?;
        Ok(Self { inner: editor })
    }

    /// Get the package name
    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// Set the package name
    #[setter]
    fn set_name(&mut self, name: String) {
        self.inner.set_name(name);
    }

    /// Get the package version
    #[getter]
    fn version(&self) -> &str {
        self.inner.version()
    }

    /// Set the package version
    #[setter]
    fn set_version(&mut self, version: String) {
        self.inner.set_version(version);
    }

    /// Get the package summary
    #[getter]
    fn summary(&self) -> Option<&str> {
        self.inner.summary()
    }

    /// Set the package summary
    #[setter]
    fn set_summary(&mut self, summary: String) {
        self.inner.set_summary(summary);
    }

    /// Get the package description
    #[getter]
    fn description(&self) -> Option<&str> {
        self.inner.description()
    }

    /// Set the package description
    #[setter]
    fn set_description(&mut self, description: String) {
        self.inner.set_description(description);
    }

    /// Get the package author
    #[getter]
    fn author(&self) -> Option<&str> {
        self.inner.author()
    }

    /// Set the package author
    #[setter]
    fn set_author(&mut self, author: String) {
        self.inner.set_author(author);
    }

    /// Get the author email
    #[getter]
    fn author_email(&self) -> Option<&str> {
        self.inner.author_email()
    }

    /// Set the author email
    #[setter]
    fn set_author_email(&mut self, email: String) {
        self.inner.set_author_email(email);
    }

    /// Get the package license
    #[getter]
    fn license(&self) -> Option<&str> {
        self.inner.license()
    }

    /// Set the package license
    #[setter]
    fn set_license(&mut self, license: String) {
        self.inner.set_license(license);
    }

    /// Get the Python version requirement
    #[getter]
    fn requires_python(&self) -> Option<&str> {
        self.inner.requires_python()
    }

    /// Set the Python version requirement
    #[setter]
    fn set_requires_python(&mut self, version: String) {
        self.inner.set_requires_python(version);
    }

    /// Get the package classifiers
    #[getter]
    fn classifiers(&self) -> Vec<String> {
        self.inner.classifiers().to_vec()
    }

    /// Set the package classifiers
    #[setter]
    fn set_classifiers(&mut self, classifiers: Vec<String>) {
        self.inner.set_classifiers(classifiers);
    }

    /// Get the package dependencies (Requires-Dist)
    #[getter]
    fn requires_dist(&self) -> Vec<String> {
        self.inner.requires_dist().to_vec()
    }

    /// Set the package dependencies (Requires-Dist)
    #[setter]
    fn set_requires_dist(&mut self, deps: Vec<String>) {
        self.inner.set_requires_dist(deps);
    }

    /// Get the project URLs
    #[getter]
    fn project_urls(&self) -> Vec<String> {
        self.inner.project_urls().to_vec()
    }

    /// Set the project URLs
    #[setter]
    fn set_project_urls(&mut self, urls: Vec<String>) {
        self.inner.set_project_urls(urls);
    }

    /// Get a metadata value by key.
    ///
    /// Args:
    ///     key: The metadata field name (e.g., "Author", "License")
    ///
    /// Returns:
    ///     The value as a string for single-value fields, or a list of strings
    ///     for multi-value fields. Returns None if the field is not set.
    fn get_metadata(&self, py: Python<'_>, key: &str) -> PyResult<PyObject> {
        let metadata = self.inner.metadata();

        // Multi-value fields return lists
        let multi_value: Option<&Vec<String>> = match key {
            "Classifier" => Some(&metadata.classifiers),
            "Platform" => Some(&metadata.platform),
            "Requires-Dist" => Some(&metadata.requires_dist),
            "Requires-External" => Some(&metadata.requires_external),
            "Project-URL" => Some(&metadata.project_url),
            "Provides-Extra" => Some(&metadata.provides_extra),
            "Provides-Dist" => Some(&metadata.provides_dist),
            "Obsoletes-Dist" => Some(&metadata.obsoletes_dist),
            _ => None,
        };

        if let Some(values) = multi_value {
            let list = PyList::new(py, values)?;
            return Ok(list.into());
        }

        // Single-value fields return strings or None
        let single_value: Option<&str> = match key {
            "Metadata-Version" => Some(&metadata.metadata_version),
            "Name" => Some(&metadata.name),
            "Version" => Some(&metadata.version),
            "Summary" => metadata.summary.as_deref(),
            "Description" => metadata.description.as_deref(),
            "Description-Content-Type" => metadata.description_content_type.as_deref(),
            "Home-page" | "Home-Page" => metadata.home_page.as_deref(),
            "Download-URL" => metadata.download_url.as_deref(),
            "Author" => metadata.author.as_deref(),
            "Author-email" | "Author-Email" => metadata.author_email.as_deref(),
            "Maintainer" => metadata.maintainer.as_deref(),
            "Maintainer-email" | "Maintainer-Email" => metadata.maintainer_email.as_deref(),
            "License" => metadata.license.as_deref(),
            "Keywords" => metadata.keywords.as_deref(),
            "Requires-Python" => metadata.requires_python.as_deref(),
            _ => {
                // Check extra headers
                if let Some(values) = metadata.extra_headers.get(key) {
                    if values.len() == 1 {
                        return Ok(values[0].clone().into_pyobject(py)?.into_any().unbind());
                    } else {
                        let list = PyList::new(py, values)?;
                        return Ok(list.into());
                    }
                }
                None
            }
        };

        match single_value {
            Some(v) => Ok(v.into_pyobject(py)?.into_any().unbind()),
            None => Ok(py.None()),
        }
    }

    /// Set a metadata value by key.
    ///
    /// Args:
    ///     key: The metadata field name (e.g., "Author", "License")
    ///     value: The value to set (string for single-value fields,
    ///            list of strings for multi-value fields)
    fn set_metadata(&mut self, py: Python<'_>, key: &str, value: PyObject) -> PyResult<()> {
        let metadata = self.inner.metadata_mut();

        // Check if it's a list (multi-value field)
        if let Ok(list) = value.downcast_bound::<PyList>(py) {
            let values: Vec<String> = list.extract()?;

            match key {
                "Classifier" => metadata.classifiers = values,
                "Platform" => metadata.platform = values,
                "Requires-Dist" => metadata.requires_dist = values,
                "Requires-External" => metadata.requires_external = values,
                "Project-URL" => metadata.project_url = values,
                "Provides-Extra" => metadata.provides_extra = values,
                "Provides-Dist" => metadata.provides_dist = values,
                "Obsoletes-Dist" => metadata.obsoletes_dist = values,
                _ => {
                    metadata.extra_headers.insert(key.to_string(), values);
                }
            }
            return Ok(());
        }

        // Single value
        let str_value: String = value.extract(py)?;

        match key {
            "Metadata-Version" => metadata.metadata_version = str_value,
            "Name" => metadata.name = str_value,
            "Version" => metadata.version = str_value,
            "Summary" => metadata.summary = Some(str_value),
            "Description" => metadata.description = Some(str_value),
            "Description-Content-Type" => metadata.description_content_type = Some(str_value),
            "Home-page" | "Home-Page" => metadata.home_page = Some(str_value),
            "Download-URL" => metadata.download_url = Some(str_value),
            "Author" => metadata.author = Some(str_value),
            "Author-email" | "Author-Email" => metadata.author_email = Some(str_value),
            "Maintainer" => metadata.maintainer = Some(str_value),
            "Maintainer-email" | "Maintainer-Email" => metadata.maintainer_email = Some(str_value),
            "License" => metadata.license = Some(str_value),
            "Keywords" => metadata.keywords = Some(str_value),
            "Requires-Python" => metadata.requires_python = Some(str_value),
            _ => {
                metadata
                    .extra_headers
                    .insert(key.to_string(), vec![str_value]);
            }
        }

        Ok(())
    }

    /// Save the edited wheel with updated metadata.
    ///
    /// Args:
    ///     output_path: Path for the output wheel. If None, a temporary file
    ///                  is created and then moved to overwrite the original.
    ///
    /// Raises:
    ///     IOError: If the wheel cannot be saved
    #[pyo3(signature = (output_path = None))]
    fn save(&self, output_path: Option<&str>) -> PyResult<()> {
        match output_path {
            Some(path) => {
                self.inner.save(path)?;
                Ok(())
            }
            None => {
                // Save to a temp file, then overwrite original
                // Get the original path from the inner editor
                let original_path = self.get_wheel_path();
                let temp_path = format!("{}.tmp", original_path);
                self.inner.save(&temp_path)?;
                std::fs::rename(&temp_path, &original_path)?;
                Ok(())
            }
        }
    }

    /// Get the path to the wheel file
    fn get_wheel_path(&self) -> String {
        // Access the path from the inner struct
        // We need to expose this from WheelEditor
        // For now, we'll store it separately or access via reflection
        // Actually, WheelEditor has a private `path` field, so we need to
        // either make it public or add a getter
        // For now, let's add a method that returns the path
        self.inner.path().to_string_lossy().to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "WheelEditor(name={}, version={}, path={})",
            self.inner.name(),
            self.inner.version(),
            self.get_wheel_path()
        )
    }
}

/// Normalize a package name for use in dist-info directory names.
///
/// While PEP 503 normalizes to hyphens for PyPI URLs, dist-info directories
/// inside wheels use underscores as separators per PEP 427.
///
/// Args:
///     name: The package name to normalize
///
/// Returns:
///     Normalized name suitable for dist-info directories
#[pyfunction]
fn normalize_dist_info_name(name: &str) -> String {
    rust_normalize_dist_info_name(name)
}

/// editwheel: High-performance Python wheel metadata editor
///
/// This module provides a fast way to edit Python wheel metadata without
/// extracting and repacking the entire wheel. It achieves constant-time
/// performance by copying unchanged files as raw compressed bytes.
#[pymodule]
fn editwheel(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWheelEditor>()?;
    m.add_function(wrap_pyfunction!(normalize_dist_info_name, m)?)?;
    Ok(())
}
