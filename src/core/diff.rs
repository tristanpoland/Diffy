use crate::core::types::{DiffHunk, DiffLine, DiffLineKind, FileDiff};
use anyhow::{Context, Result};
use similar::{ChangeTag, TextDiff};
use std::path::Path;

pub struct DiffEngine;

impl DiffEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn diff_files(&self, left_path: &Path, right_path: &Path) -> Result<FileDiff> {
        let left_content = if left_path.exists() {
            Some(std::fs::read_to_string(left_path)
                .with_context(|| format!("Failed to read left file: {}", left_path.display()))?)
        } else {
            None
        };

        let right_content = if right_path.exists() {
            Some(std::fs::read_to_string(right_path)
                .with_context(|| format!("Failed to read right file: {}", right_path.display()))?)
        } else {
            None
        };

        let hunks = match (&left_content, &right_content) {
            (Some(left), Some(right)) => self.compute_diff_hunks(left, right),
            (Some(left), None) => self.create_deletion_hunks(left),
            (None, Some(right)) => self.create_addition_hunks(right),
            (None, None) => Vec::new(),
        };

        Ok(FileDiff {
            left_content,
            right_content,
            hunks,
        })
    }

    fn compute_diff_hunks(&self, left: &str, right: &str) -> Vec<DiffHunk> {
        let diff = TextDiff::from_lines(left, right);
        let context_lines = 3; // Number of context lines to show around changes
        let mut hunks = Vec::new();
        let mut current_hunk: Option<DiffHunk> = None;
        let mut old_line_no = 1u32;
        let mut new_line_no = 1u32;
        let mut context_buffer = Vec::new();

        for change in diff.iter_all_changes() {
            let line_content = change.value().trim_end_matches('\n').to_string();
            
            match change.tag() {
                ChangeTag::Equal => {
                    if let Some(ref mut hunk) = current_hunk {
                        // Add this context line to the current hunk
                        hunk.lines.push(DiffLine {
                            kind: DiffLineKind::Context,
                            content: line_content.clone(),
                            old_line_number: Some(old_line_no),
                            new_line_number: Some(new_line_no),
                        });
                        
                        // If we've collected enough context after changes, close the hunk
                        let context_after_changes = hunk.lines.iter().rev()
                            .take_while(|line| line.kind == DiffLineKind::Context)
                            .count();
                        
                        if context_after_changes >= context_lines {
                            // Keep only the required context lines
                            let changes_end = hunk.lines.len() - context_after_changes;
                            let keep_context = std::cmp::min(context_lines, context_after_changes);
                            hunk.lines.truncate(changes_end + keep_context);
                            
                            hunks.push(current_hunk.take().unwrap());
                            context_buffer.clear();
                        }
                    } else {
                        // Store potential context lines for future hunks
                        context_buffer.push((line_content, old_line_no, new_line_no));
                        if context_buffer.len() > context_lines {
                            context_buffer.remove(0);
                        }
                    }
                    old_line_no += 1;
                    new_line_no += 1;
                }
                ChangeTag::Delete => {
                    if current_hunk.is_none() {
                        // Start a new hunk, include context
                        let start_old = if context_buffer.is_empty() { 
                            old_line_no 
                        } else { 
                            context_buffer[0].1 
                        };
                        let start_new = if context_buffer.is_empty() { 
                            new_line_no 
                        } else { 
                            context_buffer[0].2 
                        };
                        
                        current_hunk = Some(DiffHunk {
                            old_start: start_old,
                            old_lines: 0,
                            new_start: start_new,
                            new_lines: 0,
                            lines: Vec::new(),
                        });
                        
                        // Add context lines
                        if let Some(ref mut hunk) = current_hunk {
                            for (content, old_no, new_no) in &context_buffer {
                                hunk.lines.push(DiffLine {
                                    kind: DiffLineKind::Context,
                                    content: content.clone(),
                                    old_line_number: Some(*old_no),
                                    new_line_number: Some(*new_no),
                                });
                            }
                        }
                        context_buffer.clear();
                    }

                    if let Some(ref mut hunk) = current_hunk {
                        hunk.lines.push(DiffLine {
                            kind: DiffLineKind::Deletion,
                            content: line_content,
                            old_line_number: Some(old_line_no),
                            new_line_number: None,
                        });
                        hunk.old_lines += 1;
                    }
                    old_line_no += 1;
                }
                ChangeTag::Insert => {
                    if current_hunk.is_none() {
                        // Start a new hunk, include context
                        let start_old = if context_buffer.is_empty() { 
                            old_line_no 
                        } else { 
                            context_buffer[0].1 
                        };
                        let start_new = if context_buffer.is_empty() { 
                            new_line_no 
                        } else { 
                            context_buffer[0].2 
                        };
                        
                        current_hunk = Some(DiffHunk {
                            old_start: start_old,
                            old_lines: 0,
                            new_start: start_new,
                            new_lines: 0,
                            lines: Vec::new(),
                        });
                        
                        // Add context lines
                        if let Some(ref mut hunk) = current_hunk {
                            for (content, old_no, new_no) in &context_buffer {
                                hunk.lines.push(DiffLine {
                                    kind: DiffLineKind::Context,
                                    content: content.clone(),
                                    old_line_number: Some(*old_no),
                                    new_line_number: Some(*new_no),
                                });
                            }
                        }
                        context_buffer.clear();
                    }

                    if let Some(ref mut hunk) = current_hunk {
                        hunk.lines.push(DiffLine {
                            kind: DiffLineKind::Addition,
                            content: line_content,
                            old_line_number: None,
                            new_line_number: Some(new_line_no),
                        });
                        hunk.new_lines += 1;
                    }
                    new_line_no += 1;
                }
            }
        }

        if let Some(hunk) = current_hunk {
            hunks.push(hunk);
        }

        hunks
    }

    fn create_deletion_hunks(&self, content: &str) -> Vec<DiffHunk> {
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return Vec::new();
        }

        let mut diff_lines = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            diff_lines.push(DiffLine {
                kind: DiffLineKind::Deletion,
                content: line.to_string(),
                old_line_number: Some((i + 1) as u32),
                new_line_number: None,
            });
        }

        vec![DiffHunk {
            old_start: 1,
            old_lines: lines.len() as u32,
            new_start: 1,
            new_lines: 0,
            lines: diff_lines,
        }]
    }

    fn create_addition_hunks(&self, content: &str) -> Vec<DiffHunk> {
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return Vec::new();
        }

        let mut diff_lines = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            diff_lines.push(DiffLine {
                kind: DiffLineKind::Addition,
                content: line.to_string(),
                old_line_number: None,
                new_line_number: Some((i + 1) as u32),
            });
        }

        vec![DiffHunk {
            old_start: 1,
            old_lines: 0,
            new_start: 1,
            new_lines: lines.len() as u32,
            lines: diff_lines,
        }]
    }

    pub fn is_binary_file(path: &Path) -> Result<bool> {
        if !path.exists() {
            return Ok(false);
        }

        let mut buffer = [0; 8192];
        let bytes_read = std::fs::File::open(path)
            .and_then(|mut file| {
                use std::io::Read;
                file.read(&mut buffer)
            })
            .unwrap_or(0);

        // Simple heuristic: if we find null bytes in the first 8KB, consider it binary
        Ok(buffer[..bytes_read].contains(&0))
    }
}