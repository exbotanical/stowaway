ROOT_DIR="$(dirname "$(readlink -f $BASH_SOURCE)")"

# shellcheck source=tests/integration/utils/shpec_utils.bash
. "$ROOT_DIR/utils/shpec_utils.bash"

# shellcheck source=scripts/utils/run_utils.bash
. "$ROOT_DIR/../../scripts/utils/run_utils.bash"

describe 'stowaway basic operations'
  alias setup='setup_test_env "basic"'
  alias teardown='cleanup_test_env'
  it 'creates symlinks for dotfiles'
    run_stowaway stow --source "$SOURCE_DIR" --target "$TARGET_DIR"

    assert_symlink_exists "$TARGET_DIR/.bashrc"
    assert_symlink_exists "$TARGET_DIR/.gitconfig"
    assert_symlink_exists "$TARGET_DIR/.vimrc"
    assert_symlink_exists "$TARGET_DIR/.config/app/config.toml"
    assert_symlink_exists "$TARGET_DIR/scripts/setup.sh"
  ti

  it 'interpolates variables correctly'
    run_stowaway stow --source "$SOURCE_DIR" --target "$TARGET_DIR"

    assert_symlink_target_contains "$TARGET_DIR/.bashrc" 'export USER="testuser"'
    assert_symlink_target_contains "$TARGET_DIR/.bashrc" 'export THEME="dark"'
    assert_symlink_target_contains "$TARGET_DIR/.bashrc" 'echo "Welcome to test-machine, testuser!"'
    assert_symlink_target_contains "$TARGET_DIR/.gitconfig" 'name = testuser'
    assert_symlink_target_contains "$TARGET_DIR/.gitconfig" 'email = testuser@example.com'
  ti

  it 'excludes files based on patterns'
    run_stowaway stow --source "$SOURCE_DIR" --target "$TARGET_DIR"

    # .tmp files should be excluded
    assert_file_not_exists "$TARGET_DIR/temp.tmp"

    # Other files should still be linked
    assert_symlink_exists "$TARGET_DIR/.bashrc"
  ti

  it 'handles dry run mode'
    alias setup='setup_test_env "dryrun"'
    alias teardown='cleanup_test_env'
    run_stowaway stow --source "$SOURCE_DIR" --target "$TARGET_DIR" --dry-run

    # No symlinks should be created in dry-run mode
    assert_file_not_exists "$TARGET_DIR/.bashrc"
    assert_file_not_exists "$TARGET_DIR/.gitconfig"
    assert_file_not_exists "$TARGET_DIR/.vimrc"
  ti

  it 'detects conflicts'
    echo "existing content" > "$TARGET_DIR/.bashrc"

    run_stowaway stow --source "$SOURCE_DIR" --target "$TARGET_DIR"
    assert unequal $? 0
  ti

  it 'preserves nested directory structure'
    run_stowaway stow --source "$SOURCE_DIR" --target "$TARGET_DIR"

    assert_symlink_exists "$TARGET_DIR/.config/app/config.toml"
    assert_symlink_exists "$TARGET_DIR/scripts/setup.sh"
    assert_is_dir "$TARGET_DIR/.config/app"
    assert_is_dir "$TARGET_DIR/scripts"
  ti

  it 'handles rollback with invalid hash'
    run_stowaway rollback "nonexistent-hash"
    assert unequal $? 0
  ti

  it 'creates new version when content changes and supports rollback'
    # Initial stow operation
    run_stowaway stow --source "$SOURCE_DIR" --target "$TARGET_DIR"

    # Verify initial symlinks exist
    assert_symlink_exists "$TARGET_DIR/.bashrc"
    assert_symlink_exists "$TARGET_DIR/.gitconfig"

    # Capture first version hash
    first_hash=$(get_current_store_hash)
    assert test -n "$first_hash"

    # Verify symlinks point to first version
    assert_symlink_points_to_store "$TARGET_DIR/.bashrc" "$first_hash"

    # Modify a source file to change content
    echo "# Modified content for version test" >> "$SOURCE_DIR/.bashrc"

    # Stow again - should create new version due to content change
    run_stowaway stow --source "$SOURCE_DIR" --target "$TARGET_DIR"

    # Capture second version hash
    second_hash=$(get_current_store_hash)
    assert test -n "$second_hash"

    # Verify hashes are different (new version created)
    assert unequal "$first_hash" "$second_hash"

    # Verify current symlinks point to new version
    assert_symlink_points_to_store "$TARGET_DIR/.bashrc" "$second_hash"
    assert_symlink_points_to_store "$TARGET_DIR/.gitconfig" "$second_hash"

    # Rollback to first version
    run_stowaway rollback "$first_hash"
    assert equal $? 0

    # Verify symlinks now point to first version after rollback
    assert_symlink_points_to_store "$TARGET_DIR/.bashrc" "$first_hash"
    assert_symlink_points_to_store "$TARGET_DIR/.gitconfig" "$first_hash"

    # Verify session file shows first hash as current after rollback
    current_hash_after_rollback=$(get_current_store_hash)
    assert equal "$current_hash_after_rollback" "$first_hash"

    # Verify the content is actually from the first version (without modification)
    bashrc_content=$(cat "$TARGET_DIR/.bashrc")
    assert test "$bashrc_content" != *"Modified content for version test"*
  ti
end_describe
