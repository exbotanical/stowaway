use std::fs;
use std::os::unix::fs as unix_fs;
use tempfile::TempDir;

fn main() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_target = temp_dir.path().join("nonexistent.txt");
    let broken_symlink = temp_dir.path().join("broken.txt");
    
    unix_fs::symlink(&nonexistent_target, &broken_symlink).unwrap();
    
    println!("broken_symlink.exists(): {}", broken_symlink.exists());
    println!("broken_symlink.is_symlink(): {}", broken_symlink.is_symlink());
    println!("broken_symlink.is_file(): {}", broken_symlink.is_file());
    println!("broken_symlink.is_dir(): {}", broken_symlink.is_dir());
}
