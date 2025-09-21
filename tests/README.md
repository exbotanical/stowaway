## Running Tests

### Prerequisites

- Docker installed
- Rust toolchain (for local development)
- shpec testing framework (automatically installed in Docker)

### Local Testing with Docker

#### Build and Run Tests

```bash
# Build the test image
docker build -t stowaway-integration-tests .

# Run the integration tests
docker run --rm stowaway-integration-tests
```

#### Interactive Testing

```bash
# Run interactive shell for debugging
docker run --rm -it stowaway-integration-tests bash

# Once inside the container, you can run:
# shpec /tests/integration/main_shpec.bash
# or
# cd /tests/scripts && ./run-all-tests.bash
```

#### Manual shpec Testing

```bash
# Run shpec tests directly
docker run --rm stowaway-integration-tests shpec /tests/integration/main_shpec.bash
```

## Adding New Tests

### 1. Create Test Function

Add a new test function to `tests/scripts/run-all-tests.bash`:

```bash
test_new_feature() {
  local test_dir="/tmp/stowaway-test-newfeature-$$"
  local source_dir="$test_dir/dotfiles"
  local target_dir="$test_dir/home"

  mkdir -p "$source_dir" "$target_dir"
  cp -r fixtures/sample-dotfiles/* "$source_dir/"

  # Test implementation
  stowaway stow --source "$source_dir" --target "$target_dir"

  # Validation
  [[ -L "$target_dir/.bashrc" ]]
}
```

### 2. Add to Test Runner

Add the test to the main function in `run-all-tests.bash`:

```bash
run_test "New Feature" test_new_feature
```

### 3. Add Docker Compose Service

Add a new service to `docker-compose.test.yml`:

```yaml
stowaway-test-newfeature:
  build:
    context: .
    dockerfile: Dockerfile.test
  container_name: stowaway-test-newfeature
  volumes:
    - ./tests/results:/home/testuser/test-workspace/results
  command: ['bash', '-c', 'source scripts/run-all-tests.bash && test_new_feature']
```

### 4. Update GitHub Actions

Add the new scenario to the matrix in `.github/workflows/integration-tests.yml`:

```yaml
strategy:
  matrix:
    scenario:
      - basic-stow
      - variable-interpolation
      - conflict-detection
      - dry-run
      - nested-directories
      - new-feature # Add here
```
