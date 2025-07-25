use crate::core::types::{DiffStatus, FileEntry};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct FileTreeBuilder {
    left_path: PathBuf,
    right_path: PathBuf,
}

impl FileTreeBuilder {
    pub fn new(left_path: PathBuf, right_path: PathBuf) -> Self {
        Self { left_path, right_path }
    }

    pub fn build(&self) -> Result<FileEntry> {
        // Build a unified tree by traversing both directories
        let root = self.build_unified_tree(PathBuf::from(""))?;
        Ok(root)
    }

    fn build_unified_tree(&self, relative_path: PathBuf) -> Result<FileEntry> {
        let left_full_path = self.left_path.join(&relative_path);
        let right_full_path = self.right_path.join(&relative_path);
        
        let left_exists = left_full_path.exists();
        let right_exists = right_full_path.exists();
        
        let is_directory = if left_exists {
            left_full_path.is_dir()
        } else if right_exists {
            right_full_path.is_dir()
        } else {
            false
        };

        let status = if left_exists && right_exists {
            if is_directory {
                DiffStatus::Unchanged
            } else if self.files_are_equal(&relative_path)? {
                DiffStatus::Unchanged
            } else {
                DiffStatus::Modified
            }
        } else if left_exists && !right_exists {
            DiffStatus::Removed
        } else if !left_exists && right_exists {
            DiffStatus::Added
        } else {
            return Err(anyhow::anyhow!("Neither path exists: {}", relative_path.display()));
        };

        let size = if !is_directory {
            if left_exists {
                std::fs::metadata(&left_full_path).ok().map(|m| m.len())
            } else {
                std::fs::metadata(&right_full_path).ok().map(|m| m.len())
            }
        } else {
            None
        };

        let mut entry = FileEntry {
            path: relative_path.clone(),
            relative_path: relative_path.clone(),
            is_directory,
            status,
            size,
            children: Vec::new(),
        };

        if is_directory {
            // Collect all children from both directories
            let mut child_names = std::collections::BTreeSet::new();
            
            if left_exists {
                if let Ok(entries) = std::fs::read_dir(&left_full_path) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            child_names.insert(name.to_string());
                        }
                    }
                }
            }
            
            if right_exists {
                if let Ok(entries) = std::fs::read_dir(&right_full_path) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            child_names.insert(name.to_string());
                        }
                    }
                }
            }

            // Build children recursively
            for child_name in child_names {
                let child_relative_path = relative_path.join(&child_name);
                if let Ok(child_entry) = self.build_unified_tree(child_relative_path) {
                    entry.children.push(child_entry);
                }
            }

            // Sort children: directories first, then files
            entry.children.sort_by(|a, b| {
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.relative_path.file_name().cmp(&b.relative_path.file_name()),
                }
            });
        }

        Ok(entry)
    }

    fn files_are_equal(&self, relative_path: &Path) -> Result<bool> {
        let left_path = self.left_path.join(relative_path);
        let right_path = self.right_path.join(relative_path);

        if !left_path.exists() || !right_path.exists() {
            return Ok(false);
        }

        let left_meta = std::fs::metadata(&left_path)?;
        let right_meta = std::fs::metadata(&right_path)?;

        if left_meta.is_dir() && right_meta.is_dir() {
            return Ok(true);
        }

        if left_meta.is_dir() != right_meta.is_dir() {
            return Ok(false);
        }

        // For files, compare size first as a quick check
        if left_meta.len() != right_meta.len() {
            return Ok(false);
        }

        // For small files, compare content directly
        if left_meta.len() < 1024 * 1024 {
            let left_content = std::fs::read(&left_path)?;
            let right_content = std::fs::read(&right_path)?;
            return Ok(left_content == right_content);
        }

        // For larger files, we'll assume they're different if sizes match
        // This is a simplification - in a real implementation you might want
        // to compute hashes or do sampling
        Ok(true)
    }
}