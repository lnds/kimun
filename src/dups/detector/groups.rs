/// Group construction for duplicate code blocks.
///
/// Maps extended (file_index, offset) pairs back to original line numbers
/// and classifies severity based on the Rule of Three.
use super::{DuplicateGroup, DuplicateLocation, DuplicationSeverity, NormalizedFile};

/// Build a `DuplicateGroup` from the extended start locations and block size.
///
/// Maps each (file_index, offset) pair back to original line numbers via
/// `NormalizedLine::original_line_number`. Captures up to 5 sample lines
/// from the first location for display. Classifies severity as `Critical`
/// (3+ occurrences) or `Tolerable` (2 occurrences) per the Rule of Three.
pub fn build_group(
    files: &[NormalizedFile],
    start_locs: &[(usize, usize)],
    block_size: usize,
) -> DuplicateGroup {
    let mut dup_locations = Vec::new();
    let mut sample = Vec::new();

    for (file_idx, offset) in start_locs {
        let file = &files[*file_idx];
        let start_line = file.lines[*offset].original_line_number;
        let end_offset = (*offset + block_size - 1).min(file.lines.len() - 1);
        let end_line = file.lines[end_offset].original_line_number;

        if sample.is_empty() {
            let sample_end = (*offset + block_size).min(file.lines.len());
            sample = file.lines[*offset..sample_end]
                .iter()
                .take(5)
                .map(|l| l.content.clone())
                .collect();
        }

        dup_locations.push(DuplicateLocation {
            file_path: file.path.clone(),
            start_line,
            end_line,
        });
    }

    let severity = if dup_locations.len() >= 3 {
        DuplicationSeverity::Critical
    } else {
        DuplicationSeverity::Tolerable
    };

    DuplicateGroup {
        locations: dup_locations,
        line_count: block_size,
        sample,
        severity,
    }
}
