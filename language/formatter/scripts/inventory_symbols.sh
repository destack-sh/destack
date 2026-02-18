#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-language/formatter/src}"
OUT_DIR="${2:-language/formatter/.inventory}"

FILES_LIST="$OUT_DIR/files.list"
FILES_TSV="$OUT_DIR/files.tsv"
TYPES_TSV="$OUT_DIR/types.tsv"
IMPLS_TSV="$OUT_DIR/impls.tsv"
FUNCTIONS_TSV="$OUT_DIR/functions.tsv"
SUMMARY_TXT="$OUT_DIR/summary.txt"
REPORT_MD="$OUT_DIR/report.md"

mkdir -p "$OUT_DIR"

# file inventory
rg --files "$ROOT" -g '*.rs' | sort >"$FILES_LIST"

{
  printf "file\tlines\n"
  while IFS= read -r file; do
    line_count="$(wc -l <"$file" | tr -d ' ')"
    printf "%s\t%s\n" "$file" "$line_count"
  done <"$FILES_LIST"
} >"$FILES_TSV"

# type inventory
{
  printf "file\tline\tvisibility\tkind\tname\tsignature\n"
  rg --no-heading --line-number --color=never --glob '*.rs' \
    '^[[:space:]]*(pub(\([^)]*\))?[[:space:]]+)?(struct|enum|trait|type)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*' \
    "$ROOT" | awk -F: '
      {
        file=$1
        line=$2
        text=$0
        sub(/^[^:]+:[0-9]+:/, "", text)
        gsub(/^[ \t]+/, "", text)

        visibility="private"
        if (text ~ /^pub([[:space:]]|\()/) {
          visibility="pub"
        }

        kind=""
        name=""
        if (match(text, /(struct|enum|trait|type)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*/)) {
          decl=substr(text, RSTART, RLENGTH)
          split(decl, parts, /[[:space:]]+/)
          kind=parts[1]
          name=parts[2]
        }

        if (kind != "" && name != "") {
          printf "%s\t%s\t%s\t%s\t%s\t%s\n", file, line, visibility, kind, name, text
        }
      }
    '
} >"$TYPES_TSV"

# impl inventory
{
  printf "file\tline\theader\n"
  rg --no-heading --line-number --color=never --glob '*.rs' \
    '^[[:space:]]*impl([[:space:]]*<[^>]+>)?[[:space:]].*\{' \
    "$ROOT" | awk -F: '
      {
        file=$1
        line=$2
        text=$0
        sub(/^[^:]+:[0-9]+:/, "", text)
        gsub(/^[ \t]+/, "", text)
        printf "%s\t%s\t%s\n", file, line, text
      }
    '
} >"$IMPLS_TSV"

# function and method inventory
{
  printf "file\tline\tvisibility\tkind\tname\tsignature\n"
  rg --no-heading --line-number --color=never --glob '*.rs' \
    '^[[:space:]]*(pub(\([^)]*\))?[[:space:]]+)?(async[[:space:]]+)?(const[[:space:]]+)?(unsafe[[:space:]]+)?fn[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\(' \
    "$ROOT" | awk -F: '
      {
        file=$1
        line=$2
        text=$0
        sub(/^[^:]+:[0-9]+:/, "", text)
        gsub(/^[ \t]+/, "", text)

        visibility="private"
        if (text ~ /^pub([[:space:]]|\()/) {
          visibility="pub"
        }

        kind="function"
        if (text ~ /\([^)]*([&*]?[[:space:]]*mut[[:space:]]+)?self([[:space:],)]|$)/) {
          kind="method"
        }

        name=""
        if (match(text, /fn[[:space:]]+[A-Za-z_][A-Za-z0-9_]*/)) {
          decl=substr(text, RSTART, RLENGTH)
          split(decl, parts, /[[:space:]]+/)
          name=parts[2]
        }

        if (name != "") {
          printf "%s\t%s\t%s\t%s\t%s\t%s\n", file, line, visibility, kind, name, text
        }
      }
    '
} >"$FUNCTIONS_TSV"

file_count="$(($(wc -l <"$FILES_LIST") + 0))"
total_lines="$(awk -F'\t' 'NR>1 {sum += $2} END {print sum + 0}' "$FILES_TSV")"
type_count="$(awk 'END {print NR - 1}' "$TYPES_TSV")"
impl_count="$(awk 'END {print NR - 1}' "$IMPLS_TSV")"
fn_count="$(awk 'END {print NR - 1}' "$FUNCTIONS_TSV")"
method_count="$(awk -F'\t' 'NR>1 && $4=="method" {count++} END {print count + 0}' "$FUNCTIONS_TSV")"
free_fn_count="$(awk -F'\t' 'NR>1 && $4=="function" {count++} END {print count + 0}' "$FUNCTIONS_TSV")"

{
  printf "root: %s\n" "$ROOT"
  printf "files: %s\n" "$file_count"
  printf "lines: %s\n" "$total_lines"
  printf "types: %s\n" "$type_count"
  printf "impls: %s\n" "$impl_count"
  printf "functions_total: %s\n" "$fn_count"
  printf "methods: %s\n" "$method_count"
  printf "free_functions: %s\n" "$free_fn_count"
} >"$SUMMARY_TXT"

{
  printf "# Formatter Inventory\n\n"
  printf 'Root: `%s`\n\n' "$ROOT"
  printf "## Summary\n\n"
  cat "$SUMMARY_TXT" | sed 's/^/- /'
  printf "\n## Largest Files\n\n"
  printf "| File | Lines |\n"
  printf "| --- | ---: |\n"
  awk -F'\t' 'NR>1 {print $0}' "$FILES_TSV" | sort -t$'\t' -k2,2nr | head -n 30 | \
    awk -F'\t' '{printf "| `%s` | %s |\n", $1, $2}'
  printf "\n## Type Counts By Kind\n\n"
  printf "| Kind | Count |\n"
  printf "| --- | ---: |\n"
  awk -F'\t' 'NR>1 {count[$4]++} END {for (kind in count) printf "%s\t%s\n", kind, count[kind]}' "$TYPES_TSV" | \
    sort | awk -F'\t' '{printf "| `%s` | %s |\n", $1, $2}'
  printf "\n## Function Counts By Kind\n\n"
  printf "| Kind | Count |\n"
  printf "| --- | ---: |\n"
  awk -F'\t' 'NR>1 {count[$4]++} END {for (kind in count) printf "%s\t%s\n", kind, count[kind]}' "$FUNCTIONS_TSV" | \
    sort | awk -F'\t' '{printf "| `%s` | %s |\n", $1, $2}'
  printf "\n## Artifact Paths\n\n"
  printf -- '- `%s`\n' "$FILES_TSV"
  printf -- '- `%s`\n' "$TYPES_TSV"
  printf -- '- `%s`\n' "$IMPLS_TSV"
  printf -- '- `%s`\n' "$FUNCTIONS_TSV"
  printf -- '- `%s`\n' "$SUMMARY_TXT"
} >"$REPORT_MD"

printf "wrote inventory to %s\n" "$OUT_DIR"
printf "report: %s\n" "$REPORT_MD"
