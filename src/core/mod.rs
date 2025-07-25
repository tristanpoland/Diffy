pub mod diff;
pub mod tree;
pub mod types;

use crate::core::diff::DiffEngine;
use crate::core::tree::FileTreeBuilder;
use crate::core::types::{DiffResult, DiffStatus, FileEntry};
use anyhow::Result;
use rayon::prelude::*;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone)]
pub struct DiffyCore {
    pub left_path: PathBuf,
    pub right_path: PathBuf,
    pub include_ignored: bool,
}

impl DiffyCore {
    pub fn new(left_path: PathBuf, right_path: PathBuf) -> Self {
        Self { left_path, right_path, include_ignored: false }
    }

    pub fn new_with_options(left_path: PathBuf, right_path: PathBuf, include_ignored: bool) -> Self {
        Self { left_path, right_path, include_ignored }
    }

    pub fn analyze(&self) -> Result<DiffResult> {
        let start_time = Instant::now();
        println!("🔍 Analyzing directories...");
        
        let tree_builder = FileTreeBuilder::new_with_options(
            self.left_path.clone(), 
            self.right_path.clone(),
            self.include_ignored
        );
        let tree = tree_builder.build()?;
        
        let (total_files, added_count, removed_count, modified_count) = 
            Self::count_file_stats(&tree);

        let duration = start_time.elapsed();
        println!("✅ Analysis complete! {} files processed in {:.2}s", 
                total_files, duration.as_secs_f64());
        println!("   📊 {} added, {} removed, {} modified", 
                added_count, removed_count, modified_count);

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

    pub fn analyze_with_progress<F>(&self, mut progress_callback: F) -> Result<DiffResult>
    where
        F: FnMut(usize, usize) + Send + Sync,
    {
        let start_time = Instant::now();
        println!("🔍 Analyzing directories with progress tracking...");
        
        // Use a custom tree builder that reports progress
        let tree_builder = FileTreeBuilder::new_with_options(
            self.left_path.clone(), 
            self.right_path.clone(),
            self.include_ignored
        );
        let tree = tree_builder.build()?;
        
        let (total_files, added_count, removed_count, modified_count) = 
            Self::count_file_stats(&tree);

        progress_callback(total_files, total_files);

        let duration = start_time.elapsed();
        println!("✅ Analysis complete! {} files processed in {:.2}s", 
                total_files, duration.as_secs_f64());

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
        // Use parallel counting for large trees
        let (total_files, added_count, removed_count, modified_count) = 
            Self::count_recursive_parallel(entry);
        
        (total_files, added_count, removed_count, modified_count)
    }

    fn count_recursive_parallel(entry: &FileEntry) -> (usize, usize, usize, usize) {
        let mut total_files = 0;
        let mut added_count = 0;
        let mut removed_count = 0;
        let mut modified_count = 0;

        if !entry.is_directory {
            total_files = 1;
            match entry.status {
                DiffStatus::Added => added_count = 1,
                DiffStatus::Removed => removed_count = 1,
                DiffStatus::Modified => modified_count = 1,
                _ => {}
            }
        }

        if !entry.children.is_empty() {
            // For directories with many children, use parallel processing
            if entry.children.len() > 10 {
                let results: Vec<(usize, usize, usize, usize)> = entry.children
                    .par_iter()
                    .map(|child| Self::count_recursive_parallel(child))
                    .collect();

                for (t, a, r, m) in results {
                    total_files += t;
                    added_count += a;
                    removed_count += r;
                    modified_count += m;
                }
            } else {
                // For small directories, use sequential processing to avoid overhead
                for child in &entry.children {
                    let (t, a, r, m) = Self::count_recursive_parallel(child);
                    total_files += t;
                    added_count += a;
                    removed_count += r;
                    modified_count += m;
                }
            }
        }

        (total_files, added_count, removed_count, modified_count)
    }
}