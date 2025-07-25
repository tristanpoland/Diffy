pub mod diff;
pub mod tree;
pub mod types;

use crate::core::diff::DiffEngine;
use crate::core::tree::FileTreeBuilder;
use crate::core::types::{DiffResult, DiffStatus, FileEntry};
use anyhow::Result;
use std::path::PathBuf;

pub struct DiffyCore {
    pub left_path: PathBuf,
    pub right_path: PathBuf,
}

impl DiffyCore {
    pub fn new(left_path: PathBuf, right_path: PathBuf) -> Self {
        Self { left_path, right_path }
    }

    pub fn analyze(&self) -> Result<DiffResult> {
        let tree_builder = FileTreeBuilder::new(self.left_path.clone(), self.right_path.clone());
        let tree = tree_builder.build()?;
        
        let (total_files, added_count, removed_count, modified_count) = 
            Self::count_file_stats(&tree);

        Ok(DiffResult {
            left_path: self.left_path.clone(),
            right_path: self.right_path.clone(),
            tree,
            total_files,
            added_count,
            removed_count,
            modified_count,
        })
    }

    pub fn get_file_diff(&self, relative_path: &std::path::Path) -> Result<crate::core::types::FileDiff> {
        let diff_engine = DiffEngine::new();
        let left_file = self.left_path.join(relative_path);
        let right_file = self.right_path.join(relative_path);
        
        diff_engine.diff_files(&left_file, &right_file)
    }

    fn count_file_stats(entry: &FileEntry) -> (usize, usize, usize, usize) {
        let total_files = 0;
        let added_count = 0;
        let removed_count = 0;
        let modified_count = 0;

        fn count_recursive(
            entry: &FileEntry,
            totals: &mut (usize, usize, usize, usize),
        ) {
            if !entry.is_directory {
                totals.0 += 1; // total_files
                match entry.status {
                    DiffStatus::Added => totals.1 += 1,
                    DiffStatus::Removed => totals.2 += 1,
                    DiffStatus::Modified => totals.3 += 1,
                    _ => {}
                }
            }

            for child in &entry.children {
                count_recursive(child, totals);
            }
        }

        let mut totals = (total_files, added_count, removed_count, modified_count);
        count_recursive(entry, &mut totals);
        totals
    }
}