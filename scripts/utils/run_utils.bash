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

log_info() {
  blue "[INFO] $1"
}

log_success() {
  green "[SUCCESS] $1"
}

log_error() {
  red "[ERROR] $1"
}

log_warning() {
  yellow "[WARN] $1"
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
  run_stowaway unstow || true

  log_info "Cleaning up test environment..."
  rm -rf /tmp/stowaway-test-* 2> /dev/null || true
  rm -rf "$HOME"/.stowaway 2> /dev/null || true

  # Remove any existing symlinks from previous tests
  find "$HOME" -maxdepth 3 -type l -delete 2> /dev/null || true
}

# Stowaway test helpers
run_stowaway() {
  stowaway "$@" --log-level debug
}
