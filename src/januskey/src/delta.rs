// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// Delta Storage: Efficient storage for file modifications
// Stores only the differences when modifying large files

use serde::{Deserialize, Serialize};

/// Threshold: use delta if original file is larger than this
const DELTA_THRESHOLD: usize = 4096; // 4KB

/// Maximum delta size as percentage of original (if delta is larger, store full content)
const MAX_DELTA_RATIO: f64 = 0.5; // 50%

/// A delta representing changes between two versions of content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    /// Type of delta encoding used
    pub encoding: DeltaEncoding,
    /// The delta data
    pub data: Vec<u8>,
    /// Original content size (for validation)
    pub original_size: usize,
    /// New content size (for validation)
    pub new_size: usize,
}

/// Type of delta encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeltaEncoding {
    /// Full content (no delta, used when delta would be larger)
    Full,
    /// Line-based diff for text files
    LineDiff,
    /// Block-based diff for binary files
    BlockDiff,
}

/// A single edit operation in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditOp {
    /// Keep N bytes from original at position
    Keep { offset: usize, len: usize },
    /// Insert new bytes
    Insert { data: Vec<u8> },
    /// Delete N bytes from original at position
    Delete { offset: usize, len: usize },
}

/// Line-based diff for text files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDiff {
    /// List of hunks (groups of changes)
    pub hunks: Vec<DiffHunk>,
}

/// A hunk in a line diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    /// Starting line in original (0-indexed)
    pub original_start: usize,
    /// Number of lines in original
    pub original_count: usize,
    /// Starting line in new (0-indexed)
    pub new_start: usize,
    /// Number of lines in new
    pub new_count: usize,
    /// The changed lines
    pub lines: Vec<DiffLine>,
}

/// A line in a diff hunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffLine {
    /// Line exists in both (context)
    Context(String),
    /// Line only in original (removed)
    Removed(String),
    /// Line only in new (added)
    Added(String),
}

impl Delta {
    /// Compute delta between original and new content
    pub fn compute(original: &[u8], new: &[u8]) -> Self {
        // If original is small, just store full content
        if original.len() < DELTA_THRESHOLD {
            return Self::full(new);
        }

        // Try to detect if content is text
        let is_text = is_likely_text(original) && is_likely_text(new);

        if is_text {
            // Use line-based diff for text
            if let Some(delta) = Self::compute_line_diff(original, new) {
                // Check if delta is smaller than threshold
                let delta_size = delta.data.len();
                let max_size = (original.len() as f64 * MAX_DELTA_RATIO) as usize;

                if delta_size < max_size && delta_size < new.len() {
                    return delta;
                }
            }
        } else {
            // Use block-based diff for binary
            if let Some(delta) = Self::compute_block_diff(original, new) {
                let delta_size = delta.data.len();
                let max_size = (original.len() as f64 * MAX_DELTA_RATIO) as usize;

                if delta_size < max_size && delta_size < new.len() {
                    return delta;
                }
            }
        }

        // Fall back to full content
        Self::full(new)
    }

    /// Create a delta that stores full content
    fn full(content: &[u8]) -> Self {
        Self {
            encoding: DeltaEncoding::Full,
            data: content.to_vec(),
            original_size: 0,
            new_size: content.len(),
        }
    }

    /// Compute line-based diff
    fn compute_line_diff(original: &[u8], new: &[u8]) -> Option<Self> {
        let original_str = std::str::from_utf8(original).ok()?;
        let new_str = std::str::from_utf8(new).ok()?;

        let original_lines: Vec<&str> = original_str.lines().collect();
        let new_lines: Vec<&str> = new_str.lines().collect();

        // Simple LCS-based diff
        let diff = compute_lcs_diff(&original_lines, &new_lines);

        let serialized = serde_json::to_vec(&diff).ok()?;

        Some(Self {
            encoding: DeltaEncoding::LineDiff,
            data: serialized,
            original_size: original.len(),
            new_size: new.len(),
        })
    }

    /// Compute block-based diff for binary files
    fn compute_block_diff(original: &[u8], new: &[u8]) -> Option<Self> {
        const BLOCK_SIZE: usize = 64;

        let mut ops: Vec<EditOp> = Vec::new();
        let mut new_pos = 0;
        let mut orig_pos = 0;

        // Simple block matching algorithm
        while new_pos < new.len() {
            // Try to find a matching block in original
            let remaining_new = &new[new_pos..];
            let block_len = remaining_new.len().min(BLOCK_SIZE);
            let block = &remaining_new[..block_len];

            if let Some(match_pos) = find_block(original, orig_pos, block) {
                // Found a match
                if match_pos > orig_pos {
                    // There's a gap in original (deleted content)
                    ops.push(EditOp::Delete {
                        offset: orig_pos,
                        len: match_pos - orig_pos,
                    });
                }
                ops.push(EditOp::Keep {
                    offset: match_pos,
                    len: block_len,
                });
                orig_pos = match_pos + block_len;
                new_pos += block_len;
            } else {
                // No match, this is inserted content
                // Find how much is inserted before next match
                let insert_end = find_next_match(original, orig_pos, &new[new_pos..], BLOCK_SIZE)
                    .unwrap_or(new.len() - new_pos);

                ops.push(EditOp::Insert {
                    data: new[new_pos..new_pos + insert_end].to_vec(),
                });
                new_pos += insert_end;
            }
        }

        // Handle remaining original content (deleted)
        if orig_pos < original.len() {
            ops.push(EditOp::Delete {
                offset: orig_pos,
                len: original.len() - orig_pos,
            });
        }

        let serialized = serde_json::to_vec(&ops).ok()?;

        Some(Self {
            encoding: DeltaEncoding::BlockDiff,
            data: serialized,
            original_size: original.len(),
            new_size: new.len(),
        })
    }

    /// Apply delta to original content to get new content
    pub fn apply(&self, original: &[u8]) -> Option<Vec<u8>> {
        match self.encoding {
            DeltaEncoding::Full => Some(self.data.clone()),
            DeltaEncoding::LineDiff => self.apply_line_diff(original),
            DeltaEncoding::BlockDiff => self.apply_block_diff(original),
        }
    }

    /// Apply line diff
    fn apply_line_diff(&self, original: &[u8]) -> Option<Vec<u8>> {
        let diff: LineDiff = serde_json::from_slice(&self.data).ok()?;
        let original_str = std::str::from_utf8(original).ok()?;
        let original_lines: Vec<&str> = original_str.lines().collect();

        let mut result_lines: Vec<String> = Vec::new();
        let mut orig_line = 0;

        for hunk in &diff.hunks {
            // Add unchanged lines before this hunk
            while orig_line < hunk.original_start {
                if orig_line < original_lines.len() {
                    result_lines.push(original_lines[orig_line].to_string());
                }
                orig_line += 1;
            }

            // Process hunk
            for line in &hunk.lines {
                match line {
                    DiffLine::Context(s) | DiffLine::Added(s) => {
                        result_lines.push(s.clone());
                    }
                    DiffLine::Removed(_) => {
                        // Skip removed lines
                    }
                }
            }

            orig_line = hunk.original_start + hunk.original_count;
        }

        // Add remaining original lines
        while orig_line < original_lines.len() {
            result_lines.push(original_lines[orig_line].to_string());
            orig_line += 1;
        }

        // Preserve original line endings
        let line_ending = if original_str.contains("\r\n") { "\r\n" } else { "\n" };
        let result = result_lines.join(line_ending);

        // Add final newline if original had one
        let final_result = if original_str.ends_with('\n') || original_str.ends_with("\r\n") {
            format!("{}{}", result, line_ending)
        } else {
            result
        };

        Some(final_result.into_bytes())
    }

    /// Apply block diff
    fn apply_block_diff(&self, original: &[u8]) -> Option<Vec<u8>> {
        let ops: Vec<EditOp> = serde_json::from_slice(&self.data).ok()?;
        let mut result = Vec::with_capacity(self.new_size);

        for op in ops {
            match op {
                EditOp::Keep { offset, len } => {
                    if offset + len <= original.len() {
                        result.extend_from_slice(&original[offset..offset + len]);
                    }
                }
                EditOp::Insert { data } => {
                    result.extend_from_slice(&data);
                }
                EditOp::Delete { .. } => {
                    // Deletions are handled implicitly by not copying
                }
            }
        }

        Some(result)
    }

    /// Check if this delta uses full content storage
    pub fn is_full(&self) -> bool {
        self.encoding == DeltaEncoding::Full
    }

    /// Get the stored data (for content store)
    pub fn into_bytes(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap_or(self.data)
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        serde_json::from_slice(data).ok()
    }
}

/// Check if content is likely text (not binary)
fn is_likely_text(content: &[u8]) -> bool {
    if content.is_empty() {
        return true;
    }

    // Check first 8KB for null bytes or high ratio of non-printable chars
    let sample_size = content.len().min(8192);
    let sample = &content[..sample_size];

    let mut non_text_count = 0;
    for &byte in sample {
        if byte == 0 {
            return false; // Null byte = binary
        }
        if byte < 32 && byte != b'\t' && byte != b'\n' && byte != b'\r' {
            non_text_count += 1;
        }
    }

    // If more than 10% non-text characters, consider it binary
    (non_text_count as f64 / sample_size as f64) < 0.1
}

/// Compute LCS-based diff between two lists of lines
fn compute_lcs_diff(original: &[&str], new: &[&str]) -> LineDiff {
    // Simple Myers diff algorithm implementation
    let mut hunks = Vec::new();

    let (orig_len, new_len) = (original.len(), new.len());

    // Build edit graph using simple DP
    let mut lcs = vec![vec![0usize; new_len + 1]; orig_len + 1];

    for i in 1..=orig_len {
        for j in 1..=new_len {
            if original[i - 1] == new[j - 1] {
                lcs[i][j] = lcs[i - 1][j - 1] + 1;
            } else {
                lcs[i][j] = lcs[i - 1][j].max(lcs[i][j - 1]);
            }
        }
    }

    // Backtrack to find differences
    let mut i = orig_len;
    let mut j = new_len;
    let mut changes: Vec<(usize, usize, DiffLine)> = Vec::new();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && original[i - 1] == new[j - 1] {
            changes.push((i - 1, j - 1, DiffLine::Context(original[i - 1].to_string())));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || lcs[i][j - 1] >= lcs[i - 1][j]) {
            changes.push((i, j - 1, DiffLine::Added(new[j - 1].to_string())));
            j -= 1;
        } else if i > 0 {
            changes.push((i - 1, j, DiffLine::Removed(original[i - 1].to_string())));
            i -= 1;
        }
    }

    changes.reverse();

    // Group changes into hunks
    let mut current_hunk: Option<DiffHunk> = None;
    let context_lines = 3;

    for (orig_idx, new_idx, line) in changes {
        match &line {
            DiffLine::Context(_) => {
                if let Some(ref mut hunk) = current_hunk {
                    hunk.lines.push(line);
                    hunk.original_count = orig_idx - hunk.original_start + 1;
                    hunk.new_count = new_idx - hunk.new_start + 1;
                }
            }
            DiffLine::Added(_) | DiffLine::Removed(_) => {
                if current_hunk.is_none() {
                    let start_orig = orig_idx.saturating_sub(context_lines);
                    let start_new = new_idx.saturating_sub(context_lines);
                    current_hunk = Some(DiffHunk {
                        original_start: start_orig,
                        original_count: 1,
                        new_start: start_new,
                        new_count: 1,
                        lines: Vec::new(),
                    });
                }
                if let Some(ref mut hunk) = current_hunk {
                    hunk.lines.push(line);
                    hunk.original_count = orig_idx - hunk.original_start + 1;
                    hunk.new_count = new_idx - hunk.new_start + 1;
                }
            }
        }
    }

    if let Some(hunk) = current_hunk {
        hunks.push(hunk);
    }

    LineDiff { hunks }
}

/// Find a block in the original content starting from a position
fn find_block(original: &[u8], start: usize, block: &[u8]) -> Option<usize> {
    if block.is_empty() || start >= original.len() {
        return None;
    }

    let search_range = &original[start..];
    for i in 0..search_range.len().saturating_sub(block.len() - 1) {
        if search_range[i..].starts_with(block) {
            return Some(start + i);
        }
    }
    None
}

/// Find how much content before the next matching block
fn find_next_match(original: &[u8], orig_start: usize, new_content: &[u8], block_size: usize) -> Option<usize> {
    for i in 1..new_content.len() {
        let remaining = &new_content[i..];
        if remaining.len() >= block_size {
            let block = &remaining[..block_size];
            if find_block(original, orig_start, block).is_some() {
                return Some(i);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_likely_text() {
        assert!(is_likely_text(b"Hello, world!"));
        assert!(is_likely_text(b"Line 1\nLine 2\nLine 3"));
        assert!(!is_likely_text(b"\x00\x01\x02\x03"));
        assert!(is_likely_text(b""));
    }

    #[test]
    fn test_full_delta() {
        let original = b"small";
        let new = b"new content";

        let delta = Delta::compute(original, new);
        assert!(delta.is_full());
        assert_eq!(delta.apply(original).unwrap(), new.to_vec());
    }

    #[test]
    fn test_line_diff() {
        let original = b"line 1\nline 2\nline 3\nline 4\nline 5\n".repeat(100);
        let new = original.clone();
        // Modify line 50
        let new_str = String::from_utf8(new.clone()).unwrap();
        let lines: Vec<&str> = new_str.lines().collect();
        let mut new_lines = lines.clone();
        new_lines[49] = "modified line 50";
        let new_content = new_lines.join("\n") + "\n";

        let delta = Delta::compute(&original, new_content.as_bytes());

        // Delta should be smaller than original
        if !delta.is_full() {
            assert!(delta.data.len() < original.len());
        }

        // Applying delta should produce new content
        let restored = delta.apply(&original).unwrap();
        assert_eq!(restored, new_content.as_bytes());
    }

    #[test]
    fn test_delta_roundtrip() {
        let original = b"Original content here\nWith multiple lines\nAnd some more text\n".repeat(50);
        let new = b"Modified content here\nWith multiple lines\nAnd some different text\n".repeat(50);

        let delta = Delta::compute(&original, &new);
        let restored = delta.apply(&original).unwrap();

        assert_eq!(restored, new.to_vec());
    }
}
