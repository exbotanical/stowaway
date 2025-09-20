use crate::{context::StowawayContext, error::{Result, StowawayError}, file_entry::FileEntry, lifecycle::LifecyclePhase};
use walkdir::WalkDir;
use glob::Pattern;

pub struct ScanPhase;
impl LifecyclePhase for ScanPhase {
    fn execute(&self, context: &mut StowawayContext) -> Result<()> {
        println!("Scanning files in {:?}", context.source_dir);

        let include_patterns: Vec<Pattern> = context.config.interpolation.include_patterns
            .iter()
            .map(|p| Pattern::new(p))
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let exclude_patterns: Vec<Pattern> = context.config.interpolation.exclude_patterns
            .iter()
            .map(|p| Pattern::new(p))
            .collect::<std::result::Result<Vec<_>, _>>()?;

        for entry in WalkDir::new(&context.source_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let relative_path = entry.path().strip_prefix(&context.source_dir)
                    .map_err(|_| StowawayError::Config("Invalid source path".to_string()))?;

                let relative_str = relative_path.to_string_lossy();

                // Skip if matches exclude patterns
                if exclude_patterns.iter().any(|p| p.matches(&relative_str)) {
                    continue;
                }

                // Check if should be interpolated
                let should_interpolate = include_patterns.iter().any(|p| p.matches(&relative_str));

                let target_path = context.target_dir.join(relative_path);

                let file_entry = FileEntry {
                    source_path: entry.path().to_path_buf(),
                    relative_path: relative_path.to_path_buf(),
                    target_path,
                    should_interpolate,
                };

                context.files.push(file_entry);
            }
        }

        println!("Found {} files to process", context.files.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{StowawayConfig, InterpolationConfig};
    use crate::context::StowawayContext;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_context(temp_dir: &TempDir, include_patterns: Vec<String>, exclude_patterns: Vec<String>) -> StowawayContext {
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        let config = StowawayConfig {
            variables: HashMap::new(),
            interpolation: InterpolationConfig {
                include_patterns,
                exclude_patterns,
            },
        };

        StowawayContext {
            config,
            source_dir,
            target_dir,
            dry_run: false,
            files: Vec::new(),
            store_hash: None,
        }
    }

    fn create_test_files(source_dir: &std::path::Path, files: Vec<(&str, &str)>) {
        for (path, content) in files {
            let file_path = source_dir.join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&file_path, content).unwrap();
        }
    }

    #[test]
    fn test_collects_include_files() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir, vec!["*.txt".to_string()], vec![]);

        create_test_files(&context.source_dir, vec![
            ("config.txt", "config content"),
            ("readme.md", "readme content"),
            ("script.sh", "script content"),
        ]);

        let phase = ScanPhase;
        phase.execute(&mut context).unwrap();

        assert_eq!(context.files.len(), 3);

        let txt_file = context.files.iter().find(|f| f.relative_path.to_string_lossy() == "config.txt").unwrap();
        assert!(txt_file.should_interpolate);

        let md_file = context.files.iter().find(|f| f.relative_path.to_string_lossy() == "readme.md").unwrap();
        assert!(!md_file.should_interpolate);

        let sh_file = context.files.iter().find(|f| f.relative_path.to_string_lossy() == "script.sh").unwrap();
        assert!(!sh_file.should_interpolate);
    }

    #[test]
    fn test_ignores_exclude_files() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir, vec!["**/*".to_string()], vec!["*.log".to_string(), "temp/*".to_string()]);

        create_test_files(&context.source_dir, vec![
            ("config.txt", "config content"),
            ("debug.log", "log content"),
            ("temp/cache.dat", "cache content"),
            ("temp/session.tmp", "session content"),
            ("docs/readme.md", "readme content"),
        ]);

        let phase = ScanPhase;
        phase.execute(&mut context).unwrap();

        assert_eq!(context.files.len(), 2);

        let file_paths: Vec<String> = context.files.iter()
            .map(|f| f.relative_path.to_string_lossy().to_string())
            .collect();

        assert!(file_paths.contains(&"config.txt".to_string()));
        assert!(file_paths.contains(&"docs/readme.md".to_string()));
        assert!(!file_paths.contains(&"debug.log".to_string()));
        assert!(!file_paths.contains(&"temp/cache.dat".to_string()));
        assert!(!file_paths.contains(&"temp/session.tmp".to_string()));
    }

    #[test]
    fn test_recurses_directories() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir, vec!["**/*".to_string()], vec![]);

        create_test_files(&context.source_dir, vec![
            ("root.txt", "root content"),
            ("level1/file1.txt", "level1 content"),
            ("level1/level2/file2.txt", "level2 content"),
            ("level1/level2/level3/file3.txt", "level3 content"),
            ("another/branch/file4.txt", "branch content"),
        ]);

        let phase = ScanPhase;
        phase.execute(&mut context).unwrap();

        assert_eq!(context.files.len(), 5);

        let file_paths: Vec<String> = context.files.iter()
            .map(|f| f.relative_path.to_string_lossy().to_string())
            .collect();

        assert!(file_paths.contains(&"root.txt".to_string()));
        assert!(file_paths.contains(&"level1/file1.txt".to_string()));
        assert!(file_paths.contains(&"level1/level2/file2.txt".to_string()));
        assert!(file_paths.contains(&"level1/level2/level3/file3.txt".to_string()));
        assert!(file_paths.contains(&"another/branch/file4.txt".to_string()));

        for file_entry in &context.files {
            assert!(file_entry.should_interpolate);
            assert_eq!(file_entry.target_path, context.target_dir.join(&file_entry.relative_path));
        }
    }

    #[test]
    fn test_multiple_include_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir, vec!["*.txt".to_string(), "*.conf".to_string()], vec![]);

        create_test_files(&context.source_dir, vec![
            ("config.txt", "txt content"),
            ("app.conf", "conf content"),
            ("readme.md", "md content"),
            ("script.sh", "sh content"),
        ]);

        let phase = ScanPhase;
        phase.execute(&mut context).unwrap();

        assert_eq!(context.files.len(), 4);

        let txt_file = context.files.iter().find(|f| f.relative_path.to_string_lossy() == "config.txt").unwrap();
        assert!(txt_file.should_interpolate);

        let conf_file = context.files.iter().find(|f| f.relative_path.to_string_lossy() == "app.conf").unwrap();
        assert!(conf_file.should_interpolate);

        let md_file = context.files.iter().find(|f| f.relative_path.to_string_lossy() == "readme.md").unwrap();
        assert!(!md_file.should_interpolate);

        let sh_file = context.files.iter().find(|f| f.relative_path.to_string_lossy() == "script.sh").unwrap();
        assert!(!sh_file.should_interpolate);
    }

    #[test]
    fn test_complex_glob_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir,
            vec!["config/**/*.toml".to_string()],
            vec!["**/test_*".to_string(), "*.tmp".to_string()]
        );

        create_test_files(&context.source_dir, vec![
            ("config/app.toml", "app config"),
            ("config/db/database.toml", "db config"),
            ("config/cache/redis.toml", "redis config"),
            ("config/test_config.toml", "test config"),
            ("docs/readme.md", "readme"),
            ("temp.tmp", "temp file"),
            ("other.txt", "other file"),
        ]);

        let phase = ScanPhase;
        phase.execute(&mut context).unwrap();

        assert_eq!(context.files.len(), 5);

        let interpolated_files: Vec<String> = context.files.iter()
            .filter(|f| f.should_interpolate)
            .map(|f| f.relative_path.to_string_lossy().to_string())
            .collect();

        assert_eq!(interpolated_files.len(), 3);
        assert!(interpolated_files.contains(&"config/app.toml".to_string()));
        assert!(interpolated_files.contains(&"config/db/database.toml".to_string()));

        let all_files: Vec<String> = context.files.iter()
            .map(|f| f.relative_path.to_string_lossy().to_string())
            .collect();

        assert!(!all_files.contains(&"config/test_config.toml".to_string()));
        assert!(!all_files.contains(&"temp.tmp".to_string()));
        assert!(all_files.contains(&"docs/readme.md".to_string()));
        assert!(all_files.contains(&"other.txt".to_string()));
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir, vec!["**/*".to_string()], vec![]);

        let phase = ScanPhase;
        phase.execute(&mut context).unwrap();

        assert_eq!(context.files.len(), 0);
    }

    #[test]
    fn test_directories_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir, vec!["**/*".to_string()], vec![]);

        fs::create_dir_all(context.source_dir.join("empty_dir")).unwrap();
        fs::create_dir_all(context.source_dir.join("nested/empty")).unwrap();
        create_test_files(&context.source_dir, vec![
            ("file.txt", "content"),
        ]);

        let phase = ScanPhase;
        phase.execute(&mut context).unwrap();

        assert_eq!(context.files.len(), 1);
        assert_eq!(context.files[0].relative_path.to_string_lossy(), "file.txt");
    }

    #[test]
    fn test_invalid_glob_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir, vec!["[invalid".to_string()], vec![]);

        create_test_files(&context.source_dir, vec![
            ("test.txt", "content"),
        ]);

        let phase = ScanPhase;
        let result = phase.execute(&mut context);

        assert!(result.is_err());
    }

    #[test]
    fn test_exclude_takes_precedence() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir,
            vec!["*.txt".to_string()],
            vec!["secret.txt".to_string()]
        );

        create_test_files(&context.source_dir, vec![
            ("config.txt", "config"),
            ("secret.txt", "secret"),
            ("readme.md", "readme"),
        ]);

        let phase = ScanPhase;
        phase.execute(&mut context).unwrap();

        assert_eq!(context.files.len(), 2);

        let file_paths: Vec<String> = context.files.iter()
            .map(|f| f.relative_path.to_string_lossy().to_string())
            .collect();

        assert!(file_paths.contains(&"config.txt".to_string()));
        assert!(file_paths.contains(&"readme.md".to_string()));
        assert!(!file_paths.contains(&"secret.txt".to_string()));

        let config_file = context.files.iter().find(|f| f.relative_path.to_string_lossy() == "config.txt").unwrap();
        assert!(config_file.should_interpolate);

        let readme_file = context.files.iter().find(|f| f.relative_path.to_string_lossy() == "readme.md").unwrap();
        assert!(!readme_file.should_interpolate);
    }
}
