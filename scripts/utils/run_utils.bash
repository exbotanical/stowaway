RED='\033[0;31m'
GREEN='\033[1;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'

DEFAULT='\033[0m'

red() {
  printf "${RED}$*${DEFAULT}"
}

green() {
  printf "${GREEN}$*${DEFAULT}"
}

yellow() {
  printf "${YELLOW}$*${DEFAULT}"
}

blue() {
  printf "${BLUE}$*${DEFAULT}"
}

for_each() {
  local fn=$1
  shift
  local -a arr=("$@")

  for item in "${arr[@]}"; do
    $fn "$item"
  done
}

filter() {
  local fn=$1
  local suffix=$2
  local arg

  while read -r arg; do
    $fn "$arg" "$suffix" && echo "$arg"
  done
}

not_test_file() {
  local test=$1
  local suffix=$2
  local ret=0

  if [[ $test != *$suffix ]]; then
    return 1
  fi

  for ((i = 0; i < ${#SKIP_FILES[@]}; i++)); do
    if [[ $test == ${SKIP_FILES[i]} ]]; then
      ret=1
      break
    fi
  done

  return $ret
}

quietly_kill() {
  kill "$1" 2> /dev/null || true
}

# Test helper functions
setup_test_env() {
  local test_name="$1"
  export TEST_DIR="/tmp/stowaway-test-${test_name}-$$"
  export SOURCE_DIR="$TEST_DIR/dotfiles"
  export TARGET_DIR="$TEST_DIR/home"

  mkdir -p "$SOURCE_DIR" "$TARGET_DIR"

  shopt -s dotglob
  cp -r tests/fixtures/sample-dotfiles/* "$SOURCE_DIR/"
  shopt -u dotglob
}

cleanup_test_env() {
  if [[ -n "$TEST_DIR" && -d "$TEST_DIR" ]]; then
    rm -rf "$TEST_DIR"
  fi
}

# Stowaway test helpers
run_stowaway() {
  stowaway "$@" --log-level debug
}

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

# Version management helper functions
get_current_store_hash() {
  local session_file="$HOME/.stowaway/session.json"
  if [[ -f "$session_file" ]]; then
    # Extract hash from JSON using basic text processing
    grep '"hash"' "$session_file" | sed 's/.*"hash": *"\([^"]*\)".*/\1/'
  else
    echo ""
  fi
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
