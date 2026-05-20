#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
export LANG=C

FLAMEGRAPH_PATH="${1:-language/parser/target/flamegraphs/parse.svg}"
LIMIT="${2:-25}"

if [[ ! -f "$FLAMEGRAPH_PATH" ]]; then
  echo "missing flamegraph: $FLAMEGRAPH_PATH"
  exit 2
fi

echo "summary: $FLAMEGRAPH_PATH"
echo

echo "top non-runtime frames"
perl -ne '
  sub skip_symbol {
    my ($symbol) = @_;
    return $symbol eq "all"
      || $symbol =~ /^[0-9]+$/
      || $symbol =~ /^std::/
      || $symbol =~ /^core::/
      || $symbol =~ /^<.*core::/
      || $symbol =~ /^parse::/
      || $symbol =~ /^_main$/
      || $symbol =~ /^__mh_execute_header$/
      || $symbol =~ /^___simple_bprintf$/;
  }

  while (/<title>([^<]+)<\/title>/g) {
    $title = $1;
    $title =~ s/&lt;/</g;
    $title =~ s/&gt;/>/g;
    $title =~ s/&amp;/\&/g;

    if ($title =~ /^(.*) \((\d+) samples, (\d+(?:\.\d+)?)%\)$/) {
      next if skip_symbol($1);
      print "$3\t$2\t$1\n";
    }
  }
' "$FLAMEGRAPH_PATH" \
  | sort -nr \
  | head -n "$LIMIT" \
  | awk -F'\t' '{printf "%6.2f%%  %5s samples  %s\n", $1, $2, $3}' \
  || true

echo
echo "top parser frames"
perl -ne '
  while (/<title>([^<]+)<\/title>/g) {
    $title = $1;
    $title =~ s/&lt;/</g;
    $title =~ s/&gt;/>/g;
    $title =~ s/&amp;/\&/g;

    if ($title =~ /^(.*) \((\d+) samples, (\d+(?:\.\d+)?)%\)$/) {
      $symbol = $1;
      $samples = $2;
      $percent = $3;
      next unless $symbol =~ /destack_parser::/;
      print "$percent\t$samples\t$symbol\n";
    }
  }
' "$FLAMEGRAPH_PATH" \
  | sort -nr \
  | head -n "$LIMIT" \
  | awk -F'\t' '{printf "%6.2f%%  %5s samples  %s\n", $1, $2, $3}' \
  || true
