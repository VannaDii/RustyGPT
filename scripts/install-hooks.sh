#!/bin/sh

# Script to install Git hooks for the RustyGPT project

echo "Installing Git hooks..."

# Create scripts directory if it doesn't exist
mkdir -p scripts

# Copy the pre-push hook
echo "Installing pre-push hook..."
cp scripts/pre-push.sh .git/hooks/pre-push
chmod +x .git/hooks/pre-push
echo "Pre-push hook installed successfully."

echo "Git hooks installation complete."
