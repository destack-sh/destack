#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/../../.." && pwd)"
rules_dir="$repo_root/language/linter/src/rules"

category_prefix() {
    case "$1" in
        Complexity) echo "LX" ;;
        Correctness) echo "LC" ;;
        Performance) echo "LP" ;;
        Restriction) echo "LR" ;;
        Security) echo "LS" ;;
        Suspicious) echo "LU" ;;
        Style) echo "LY" ;;
        *)
            echo "unknown lint category: $1" >&2
            exit 1
            ;;
    esac
}

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

lint_files="$tmp_dir/lint_files.txt"
metadata="$tmp_dir/metadata.tsv"
plan="$tmp_dir/plan.tsv"
mapfile_out="$tmp_dir/map.tsv"

# collect every lint declaration file
rg -l --glob '*.rs' 'declare_lint!\s*\{' "$rules_dir" | sort > "$lint_files"

if [[ ! -s "$lint_files" ]]; then
    echo "no lint declaration files found" >&2
    exit 1
fi

while IFS= read -r file; do
    lint_id="$(rg -o 'id = "[a-z0-9-]+"' "$file" | head -n1 | sed -E 's/.*"([^"]+)".*/\1/')"
    category="$(rg -o 'category = (Complexity|Correctness|Performance|Restriction|Security|Style|Suspicious)' "$file" | head -n1 | awk '{print $3}')"
    old_code="$(rg -o 'code = "L[A-Z][0-9]{3}"' "$file" | head -n1 | sed -E 's/.*"([^"]+)".*/\1/')"

    if [[ -z "$lint_id" || -z "$category" || -z "$old_code" ]]; then
        echo "missing lint metadata in $file" >&2
        exit 1
    fi

    prefix="$(category_prefix "$category")"
    printf '%s\t%s\t%s\t%s\t%s\n' "$prefix" "$lint_id" "$file" "$old_code" "$category" >> "$metadata"
done < "$lint_files"

# build deterministic renumber plan:
# category prefix, then lint id, then file path
sort -t $'\t' -k1,1 -k2,2 -k3,3 "$metadata" |
    awk -F '\t' '
        BEGIN {
            OFS = "\t"
        }
        {
            prefix = $1
            lint_id = $2
            file = $3
            old_code = $4
            category = $5

            if (prefix != current_prefix) {
                current_prefix = prefix
                sequence = 0
            }
            sequence += 1
            new_code = sprintf("%s%03d", prefix, sequence)
            print prefix, lint_id, file, old_code, new_code, category
        }
    ' > "$plan"

if [[ ! -s "$plan" ]]; then
    echo "failed to build renumber plan" >&2
    exit 1
fi

while IFS=$'\t' read -r _prefix _lint_id file _old_code new_code _category; do
    NEW_CODE="$new_code" python3 - "$file" <<'PY'
import os
import pathlib
import re
import sys

path = pathlib.Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
new_code = os.environ["NEW_CODE"]
updated, count = re.subn(
    r'code = "L[A-Z][0-9]{3}"',
    f'code = "{new_code}"',
    text,
    count=1,
)
if count != 1:
    raise SystemExit(f"expected one code assignment in {path}")
path.write_text(updated, encoding="utf-8")
PY
done < "$plan"

# verify final codes are unique
rg -o --glob '*.rs' 'code = "L[A-Z][0-9]{3}"' "$rules_dir" |
    sed -E 's/.*"([^"]+)".*/\1/' |
    sort | uniq -d > "$tmp_dir/duplicates.txt"

if [[ -s "$tmp_dir/duplicates.txt" ]]; then
    echo "duplicate lint codes detected after renumbering:" >&2
    cat "$tmp_dir/duplicates.txt" >&2
    exit 1
fi

# emit mapping for review
awk -F '\t' '
    BEGIN {
        OFS = "\t"
        print "category", "id", "old_code", "new_code", "file"
    }
    {
        print $6, $2, $4, $5, $3
    }
' "$plan" > "$mapfile_out"

cat "$mapfile_out"
