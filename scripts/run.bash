#!/usr/bin/env bash
IFS=$'\n'

TESTING_DIR=tests/integration

# shellcheck source=scripts/utils/run_utils.bash
. "$(dirname "$(readlink -f "$BASH_SOURCE")")"/utils/run_utils.bash

declare -a SKIP_FILES=(
  # Add any files to skip here
)

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

cleanup_test_env() {
  log_info "Cleaning up test environment..."
  rm -rf /tmp/stowaway-test-* 2> /dev/null || true
  rm -rf "$HOME"/.stowaway-store 2> /dev/null || true

  # Remove any existing symlinks from previous tests
  find "$HOME" -maxdepth 3 -type l -delete 2> /dev/null || true
}

run_test() {
  local file_name="$1"
  echo "Running test: $file_name"
  shpec "$TESTING_DIR/$file_name"
}

run() {
  declare -a tests=(
    $(ls $TESTING_DIR | filter not_test_file _shpec.bash)
  )

  for_each run_test "${tests[@]}"
}

main() {
  kill $(pgrep integ_test) 2> /dev/null || :

  run
}

main "$@"
