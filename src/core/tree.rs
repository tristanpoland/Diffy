use crate::core::types::{DiffStatus, FileEntry};
use anyhow::Result;
use rayon::prelude::*;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct FileTreeBuilder {
    left_path: PathBuf,
    right_path: PathBuf,
    include_ignored: bool,
}

#[derive(Debug, Clone)]
struct FileInfo {
    path: PathBuf,
    relative_path: PathBuf,
    is_directory: bool,
    size: Option<u64>,
    exists_left: bool,
    exists_right: bool,
}

impl FileTreeBuilder {
    pub fn new(left_path: PathBuf, right_path: PathBuf) -> Self {
        Self { left_path, right_path, include_ignored: false }
    }

    pub fn new_with_options(left_path: PathBuf, right_path: PathBuf, include_ignored: bool) -> Self {
        Self { left_path, right_path, include_ignored }
    }

    pub fn build(&self) -> Result<FileEntry> {
        // Phase 1: Parallel file discovery
        let all_files = self.discover_all_files()?;
        
        // Phase 2: Parallel status computation
        let file_statuses = self.compute_file_statuses(all_files)?;
        
        // Phase 3: Build tree structure
        let root = self.build_tree_from_statuses(file_statuses)?;
        
        Ok(root)
    }

    fn discover_all_files(&self) -> Result<Vec<FileInfo>> {
        let left_files = Arc::new(Mutex::new(BTreeSet::new()));
        let right_files = Arc::new(Mutex::new(BTreeSet::new()));

        // Discover files in parallel
        let include_ignored = self.include_ignored;
        rayon::scope(|s| {
            let left_files = left_files.clone();
            let left_path = self.left_path.clone();
            s.spawn(move |_| {
                if let Ok(files) = Self::collect_files_parallel_static(&left_path, include_ignored) {
                    *left_files.lock().unwrap() = files;
                }
            });

            let right_files = right_files.clone();
            let right_path = self.right_path.clone();
            s.spawn(move |_| {
                if let Ok(files) = Self::collect_files_parallel_static(&right_path, include_ignored) {
                    *right_files.lock().unwrap() = files;
                }
            });
        });

        let left_files = left_files.lock().unwrap().clone();
        let right_files = right_files.lock().unwrap().clone();

        // Combine all unique paths
        let mut all_paths = BTreeSet::new();
        all_paths.extend(left_files.iter().cloned());
        all_paths.extend(right_files.iter().cloned());

        // Create FileInfo structs
        let file_infos: Vec<FileInfo> = all_paths
            .into_par_iter()
            .map(|relative_path| {
                let left_full_path = self.left_path.join(&relative_path);
                let right_full_path = self.right_path.join(&relative_path);
                
                let exists_left = left_full_path.exists();
                let exists_right = right_full_path.exists();
                
                let is_directory = if exists_left {
                    left_full_path.is_dir()
                } else if exists_right {
                    right_full_path.is_dir()
                } else {
                    false
                };

                let size = if !is_directory {
                    if exists_left {
                        std::fs::metadata(&left_full_path).ok().map(|m| m.len())
                    } else {
                        std::fs::metadata(&right_full_path).ok().map(|m| m.len())
                    }
                } else {
                    None
                };

                FileInfo {
                    path: relative_path.clone(),
                    relative_path,
                    is_directory,
                    size,
                    exists_left,
                    exists_right,
                }
            })
            .collect();

        Ok(file_infos)
    }

    fn collect_files_parallel_static(root: &Path, include_ignored: bool) -> Result<BTreeSet<PathBuf>> {
        if !root.exists() {
            return Ok(BTreeSet::new());
        }

        let files = Arc::new(Mutex::new(BTreeSet::new()));
        let walker = ignore::WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(!include_ignored)
            .threads(std::cmp::max(1, num_cpus::get() / 2))
            .build_parallel();

        walker.run(|| {
            let files = files.clone();
            let root = root.to_path_buf();
            Box::new(move |entry| {
                if let Ok(entry) = entry {
                    if let Ok(relative_path) = entry.path().strip_prefix(&root) {
                        if !relative_path.as_os_str().is_empty() {
                            files.lock().unwrap().insert(relative_path.to_path_buf());
                        }
                    }
                }
                ignore::WalkState::Continue
            })
        });

        Ok(Arc::try_unwrap(files).unwrap().into_inner().unwrap())
    }

    fn compute_file_statuses(&self, file_infos: Vec<FileInfo>) -> Result<HashMap<PathBuf, (FileInfo, DiffStatus)>> {
        let statuses: HashMap<PathBuf, (FileInfo, DiffStatus)> = file_infos
            .into_par_iter()
            .map(|info| {
                let status = if info.exists_left && info.exists_right {
                    if info.is_directory {
                        DiffStatus::Unchanged
                    } else if self.files_are_equal(&info.relative_path).unwrap_or(false) {
                        DiffStatus::Unchanged
                    } else {
                        DiffStatus::Modified
                    }
                } else if info.exists_left && !info.exists_right {
                    DiffStatus::Removed
                } else if !info.exists_left && info.exists_right {
                    DiffStatus::Added
                } else {
                    DiffStatus::Unchanged // Shouldn't happen
                };

                (info.relative_path.clone(), (info, status))
            })
            .collect();

        Ok(statuses)
    }

    fn build_tree_from_statuses(&self, statuses: HashMap<PathBuf, (FileInfo, DiffStatus)>) -> Result<FileEntry> {
        // Build the tree structure
        let root_info = FileInfo {
            path: PathBuf::from(""),
            relative_path: PathBuf::from(""),
            is_directory: true,
            size: None,
            exists_left: true,
            exists_right: true,
        };

        let root_entry = self.build_entry_recursive(root_info, DiffStatus::Unchanged, &statuses)?;
        Ok(root_entry)
    }

    fn build_entry_recursive(
        &self,
        info: FileInfo,
        status: DiffStatus,
        all_statuses: &HashMap<PathBuf, (FileInfo, DiffStatus)>,
    ) -> Result<FileEntry> {
        let mut entry = FileEntry {
            path: info.path.clone(),
            relative_path: info.relative_path.clone(),
            is_directory: info.is_directory,
            status,
            size: info.size,
            children: Vec::new(),
        };

        if info.is_directory {
            // Find all direct children
            let mut children: Vec<(FileInfo, DiffStatus)> = all_statuses
                .values()
                .filter_map(|(child_info, child_status)| {
                    if let Some(parent) = child_info.relative_path.parent() {
                        if parent == info.relative_path {
                            Some((child_info.clone(), child_status.clone()))
                        } else {
                            None
                        }
                    } else if info.relative_path.as_os_str().is_empty() && child_info.relative_path.components().count() == 1 {
                        Some((child_info.clone(), child_status.clone()))
                    } else {
                        None
                    }
                })
                .collect();

            // Sort children: directories first, then files
            children.sort_by(|(a_info, _), (b_info, _)| {
                match (a_info.is_directory, b_info.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a_info.relative_path.file_name().cmp(&b_info.relative_path.file_name()),
                }
            });

            // Build children recursively
            for (child_info, child_status) in children {
                if let Ok(child_entry) = self.build_entry_recursive(child_info, child_status, all_statuses) {
                    entry.children.push(child_entry);
                }
            }
        }

        Ok(entry)
    }

    fn files_are_equal(&self, relative_path: &Path) -> Result<bool> {
        let left_path = self.left_path.join(relative_path);
        let right_path = self.right_path.join(relative_path);

        if !left_path.exists() || !right_path.exists() {
            return Ok(false);
        }

        // Use parallel file comparison for efficiency
        let (left_meta, right_meta) = rayon::join(
            || std::fs::metadata(&left_path),
            || std::fs::metadata(&right_path),
        );

        let left_meta = left_meta?;
        let right_meta = right_meta?;

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

        // For small files, compare content directly in parallel
        if left_meta.len() < 1024 * 1024 {
            let (left_result, right_result) = rayon::join(
                || std::fs::read(&left_path),
                || std::fs::read(&right_path),
            );
            
            let left_content = left_result?;
            let right_content = right_result?;
            return Ok(left_content == right_content);
        }

        // For larger files, do a more sophisticated comparison
        // Compare file hashes in parallel chunks
        self.compare_large_files(&left_path, &right_path)
    }

    fn compare_large_files(&self, left_path: &Path, right_path: &Path) -> Result<bool> {
        use std::fs::File;
        use std::io::{BufReader, Read};

        const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks
        
        let (left_file, right_file) = rayon::join(
            || File::open(left_path).map(BufReader::new),
            || File::open(right_path).map(BufReader::new),
        );

        let mut left_reader = left_file?;
        let mut right_reader = right_file?;

        loop {
            let (left_chunk, right_chunk) = rayon::join(
                || {
                    let mut buffer = vec![0u8; CHUNK_SIZE];
                    left_reader.read(&mut buffer).map(|n| {
                        buffer.truncate(n);
                        buffer
                    })
                },
                || {
                    let mut buffer = vec![0u8; CHUNK_SIZE];
                    right_reader.read(&mut buffer).map(|n| {
                        buffer.truncate(n);
                        buffer
                    })
                },
            );

            let left_chunk = left_chunk?;
            let right_chunk = right_chunk?;

            if left_chunk != right_chunk {
                return Ok(false);
            }

            if left_chunk.is_empty() {
                break; // EOF reached
            }
        }

        Ok(true)
    }
}