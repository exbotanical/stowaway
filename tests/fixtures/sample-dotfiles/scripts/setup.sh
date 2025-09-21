#!/usr/bin/env bash
# Setup script for @user@
# Theme: @theme@

set -euo pipefail

USER="@user@"
EMAIL="@email@"
THEME="@theme@"
HOSTNAME="@hostname@"

echo "Setting up environment for $USER"
echo "Email: $EMAIL"
echo "Theme: $THEME"
echo "Hostname: $HOSTNAME"

# Create necessary directories
mkdir -p "$HOME/.config"
mkdir -p "$HOME/.local/bin"
mkdir -p "$HOME/.cache"

# Set up git if not already configured
if ! git config --global user.name > /dev/null 2>&1; then
  git config --global user.name "$USER"
  git config --global user.email "$EMAIL"
  echo "Git configured for $USER <$EMAIL>"
fi

# Theme-specific setup
if [ "$THEME" = "dark" ]; then
  echo "Setting up dark theme preferences..."
  export TERM_THEME="dark"
else
  echo "Setting up light theme preferences..."
  export TERM_THEME="light"
fi

echo "Setup complete for @user@ on @hostname@!"
