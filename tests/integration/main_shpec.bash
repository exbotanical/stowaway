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
    run_stowaway stow --source "$SOURCE_DIR" --target "$TARGET_DIR" --dry-run

    # No symlinks should be created in dry-run mode
    assert_file_not_exists "$TARGET_DIR/.bashrc"
    assert_file_not_exists "$TARGET_DIR/.gitconfig"
    assert_file_not_exists "$TARGET_DIR/.vimrc"
  ti

  it 'can run twice on non-mutated files just fine'
    run_stowaway stow --source "$SOURCE_DIR" --target "$TARGET_DIR"
    run_stowaway stow --source "$SOURCE_DIR" --target "$TARGET_DIR"

    assert equal $? 0
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

end_describe
