use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffStatus {
    Added,
    Removed,
    Modified,
    Unchanged,
    Conflicted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub is_directory: bool,
    pub status: DiffStatus,
    pub size: Option<u64>,
    pub children: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub left_path: PathBuf,
    pub right_path: PathBuf,
    pub tree: FileEntry,
    pub total_files: usize,
    pub added_count: usize,
    pub removed_count: usize,
    pub modified_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub left_content: Option<String>,
    pub right_content: Option<String>,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
    pub old_line_number: Option<u32>,
    pub new_line_number: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffLineKind {
    Context,
    Addition,
    Deletion,
}

impl DiffStatus {
    pub fn color_code(&self) -> &'static str {
        match self {
            DiffStatus::Added => "#00ff00",
            DiffStatus::Removed => "#ff0000", 
            DiffStatus::Modified => "#ffff00",
            DiffStatus::Unchanged => "#ffffff",
            DiffStatus::Conflicted => "#ff00ff",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DiffStatus::Added => "+",
            DiffStatus::Removed => "-",
            DiffStatus::Modified => "~",
            DiffStatus::Unchanged => " ",
            DiffStatus::Conflicted => "!",
        }
    }
}