shopt -s expand_aliases

alias it='(_shpec_failures=0; alias setup &>/dev/null && { setup; unalias setup; alias teardown &>/dev/null && trap teardown EXIT ;}; it'
# shellcheck disable=SC2154
alias ti='return "$_shpec_failures"); (( _shpec_failures += $?, _shpec_examples++ ))'
alias end_describe='end; unalias setup teardown 2>/dev/null'

assert_symlink_exists() {
  local file="$1"
  [[ -L "$file" ]]
  assert equal $? 0
}

assert_is_dir() {
  local file="$1"
  test -d "$file"
  assert equal $? 0
}

assert_file_not_exists() {
  local file="$1"
  assert file_absent "$file"
}

assert_file_contains() {
  local file="$1"
  local pattern="$2"
  [[ -f "$file" ]] && grep -q "$pattern" "$file"
}

assert_symlink_target_contains() {
  local symlink="$1"
  local pattern="$2"

  [[ -L "$symlink" ]] && {
    local target
    target=$(readlink "$symlink")
    [[ -f "$target" ]] && grep -q "$pattern" "$target"
    assert equal $? 0
    return 0
  }

  assert equal 0 -1
}

assert_symlink_points_to_store() {
  local symlink="$1"
  local expected_hash="$2"

  [[ -L "$symlink" ]] || {
    assert equal 0 -1
    return 1
  }

  local target
  target=$(readlink "$symlink")
  local expected_store_path="$HOME/.stowaway/store/$expected_hash"

  # Check if the symlink target starts with the expected store path
  [[ "$target" == "$expected_store_path"* ]]
  assert equal $? 0
}

assert_not_empty() {
  local string="$1"

  test -n "$string"
  assert equal $? 0
}
