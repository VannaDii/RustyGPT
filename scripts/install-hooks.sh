#!/bin/sh

# Script to install Git hooks for the RustyGPT project

echo "Installing Git hooks..."

# Clean up previously installed hooks first
echo "Cleaning up previously installed hooks..."
rm -f .git/hooks/pre-commit .git/hooks/pre-push
echo "Previous hooks removed."

# Create the pre-commit hook file
echo "Installing pre-commit hook..."
cat >.git/hooks/pre-commit <<'EOF'
#!/bin/sh
set -e
echo "Running pre-commit checks..."
just check
EOF

# Make the pre-commit hook executable
chmod +x .git/hooks/pre-commit
echo "Pre-commit hook installed successfully."

echo "Git hooks installation complete."
