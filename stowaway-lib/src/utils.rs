use std::path::Path;

pub fn is_member(compare_to: &[String], target: &Path) -> bool {
    compare_to.iter().any(|pattern| {
        glob::Pattern::new(pattern)
            .map(|p| p.matches_path(target))
            .unwrap_or(false)
    })
}
