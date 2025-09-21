# Sample Dotfiles for @user@

This is a test dotfiles repository for the Stowaway integration tests.

## Configuration

- **User**: @user@
- **Email**: @email@
- **Theme**: @theme@
- **Editor**: @editor@
- **Shell**: @shell@
- **Hostname**: @hostname@

## Files Included

### Shell Configuration

- `.bashrc` - Bash shell configuration with user-specific settings

### Version Control

- `.gitconfig` - Git configuration with user details and aliases

### Editor Configuration

- `.vimrc` - Vim editor configuration with theme-specific settings

### Application Configuration

- `.config/app/config.toml` - Sample application configuration

### Scripts

- `scripts/setup.sh` - Environment setup script

## Variable Interpolation

This dotfiles repository uses Stowaway's variable interpolation feature. Variables are defined in `stowaway.yaml` and referenced using the `@variable@` syntax throughout the configuration files.

## Testing Notes

This repository is designed to test:

- Variable interpolation in multiple file formats
- Nested directory structures
- Mixed file types (shell scripts, config files, etc.)
- Files that should and shouldn't be interpolated based on patterns
