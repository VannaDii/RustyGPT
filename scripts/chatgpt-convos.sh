#!/usr/bin/env bash
# save-some-convs.sh
# Usage:
#   ./save-some-convs.sh conversations.json ./out \
#       "RustyGPT" "RustyGPT – API design" "RustyGPT Playground"

set -euo pipefail

if (( $# < 3 )); then
  echo "Usage: $0 <conversations.json> <output_dir> <title1> [title2 …]" >&2
  exit 1
fi

INPUT=$1
OUTDIR=$2
shift 2                              # $@ now holds the titles

mkdir -p "$OUTDIR"

slugify() {
  iconv -t ascii//TRANSLIT <<<"$1" \
  | tr '[:upper:]' '[:lower:]' \
  | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//; s/-+/-/g'
}

for title in "$@"; do
  slug=$(slugify "$title")
  outfile="$OUTDIR/${slug}.json"

  if [[ -e "$outfile" ]]; then
    echo "Skipping conflicted conversation \"${title}\""
    continue
  fi

  # grab the conversation (jq -e makes the script exit if nothing found,
  # so we wrap it in a subshell to handle the error ourselves)
  if ! conv_json=$(jq -e --arg t "$title" \
        'first(.[] | select(.title == $t))' "$INPUT"); then
    echo "Conversation not found: \"${title}\""
    continue
  fi

  # pretty-print straight into the file
  printf '%s\n' "$conv_json" | jq '.' > "$outfile"
  echo "Saved \"${title}\" → ${outfile}"
done