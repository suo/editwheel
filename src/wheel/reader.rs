//! Wheel reader - reads metadata from Python wheels

use std::io::Read;
use std::io::Seek;

use zip::ZipArchive;

use crate::error::WheelError;
use crate::metadata::Metadata;
use crate::record::Record;
use crate::wheel_info::WheelInfo;

/// Reader for Python wheel files
pub struct WheelReader<R: Read + Seek> {
    archive: ZipArchive<R>,
    dist_info_prefix: String,
}

impl<R: Read + Seek> WheelReader<R> {
    /// Create a new wheel reader from a reader
    pub fn new(reader: R) -> Result<Self, WheelError> {
        let mut archive = ZipArchive::new(reader)?;
        let dist_info_prefix = Self::find_dist_info_prefix(&mut archive)?;

        Ok(Self {
            archive,
            dist_info_prefix,
        })
    }

    /// Find the .dist-info directory prefix
    fn find_dist_info_prefix<T: Read + Seek>(
        archive: &mut ZipArchive<T>,
    ) -> Result<String, WheelError> {
        for i in 0..archive.len() {
            let file = archive.by_index_raw(i)?;
            let name = file.name();
            if name.contains(".dist-info/") {
                let prefix = name.split(".dist-info/").next().unwrap();
                return Ok(format!("{}.dist-info", prefix));
            }
        }
        Err(WheelError::InvalidWheel(
            "No .dist-info directory found".to_string(),
        ))
    }

    /// Get the dist-info prefix (e.g., "package-1.0.0.dist-info")
    pub fn dist_info_prefix(&self) -> &str {
        &self.dist_info_prefix
    }

    /// Read and parse the METADATA file
    pub fn read_metadata(&mut self) -> Result<Metadata, WheelError> {
        let path = format!("{}/METADATA", self.dist_info_prefix);
        let mut file = self.archive.by_name(&path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Ok(Metadata::parse(&content)?)
    }

    /// Read and parse the RECORD file
    pub fn read_record(&mut self) -> Result<Record, WheelError> {
        let path = format!("{}/RECORD", self.dist_info_prefix);
        let mut file = self.archive.by_name(&path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Ok(Record::parse(&content)?)
    }

    /// Read the WHEEL file content
    pub fn read_wheel_file(&mut self) -> Result<String, WheelError> {
        let path = format!("{}/WHEEL", self.dist_info_prefix);
        let mut file = self.archive.by_name(&path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Ok(content)
    }

    /// Read and parse the WHEEL file into WheelInfo
    pub fn read_wheel_info(&mut self) -> Result<WheelInfo, WheelError> {
        let content = self.read_wheel_file()?;
        Ok(WheelInfo::parse(&content)?)
    }

    /// Get access to the underlying archive
    pub fn archive(&self) -> &ZipArchive<R> {
        &self.archive
    }

    /// Get mutable access to the underlying archive
    pub fn archive_mut(&mut self) -> &mut ZipArchive<R> {
        &mut self.archive
    }

    /// Get the number of files in the archive
    pub fn len(&self) -> usize {
        self.archive.len()
    }

    /// Check if the archive is empty
    pub fn is_empty(&self) -> bool {
        self.archive.len() == 0
    }
}
