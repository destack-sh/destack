#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
export LANG=C

FLAMEGRAPH_PATH="${1:-target/criterion/destack_parser_single/parse/single-thread/profile/flamegraph.svg}"
LIMIT="${2:-25}"

if [[ ! -f "$FLAMEGRAPH_PATH" ]]; then
  echo "missing flamegraph: $FLAMEGRAPH_PATH"
  exit 2
fi

echo "summary for: $FLAMEGRAPH_PATH"
echo

echo "top frames by raw percentage"
perl -ne '
  while (/<title>([^<]+)<\/title>/g) {
    $t = $1;
    $t =~ s/&lt;/</g;
    $t =~ s/&gt;/>/g;
    $t =~ s/&amp;/\&/g;
    if ($t =~ /(\d+(?:\.\d+)?)%\)$/) {
      print "$1\t$t\n";
    }
  }
' "$FLAMEGRAPH_PATH" | sort -nr | head -n "$LIMIT" || true

echo
echo "top symbols by max observed percentage"
perl -ne '
  while (/<title>([^<]+)<\/title>/g) {
    $t = $1;
    $t =~ s/&lt;/</g;
    $t =~ s/&gt;/>/g;
    $t =~ s/&amp;/\&/g;
    if ($t =~ /^(.*) \((\d+) samples, (\d+(?:\.\d+)?)%\)$/) {
      $symbol = $1;
      $samples = $2 + 0;
      $percent = $3 + 0;
      if (!exists $max_percent{$symbol} || $percent > $max_percent{$symbol}) {
        $max_percent{$symbol} = $percent;
        $max_samples{$symbol} = $samples;
      }
    }
  }
  END {
    for $symbol (keys %max_percent) {
      printf "%.4f\t%d\t%s\n", $max_percent{$symbol}, $max_samples{$symbol}, $symbol;
    }
  }
' "$FLAMEGRAPH_PATH" | sort -nr | head -n "$LIMIT" | awk -F'\t' '{printf "%6.2f%%  %5s samples  %s\n", $1, $2, $3}' || true

echo
echo "top parser symbols only"
perl -ne '
  while (/<title>([^<]+)<\/title>/g) {
    $t = $1;
    $t =~ s/&lt;/</g;
    $t =~ s/&gt;/>/g;
    $t =~ s/&amp;/\&/g;
    if ($t =~ /^(.*) \((\d+) samples, (\d+(?:\.\d+)?)%\)$/) {
      $symbol = $1;
      $samples = $2 + 0;
      $percent = $3 + 0;
      next unless $symbol =~ /destack_parser::parse::parser::Parser/;
      if (!exists $max_percent{$symbol} || $percent > $max_percent{$symbol}) {
        $max_percent{$symbol} = $percent;
        $max_samples{$symbol} = $samples;
      }
    }
  }
  END {
    for $symbol (keys %max_percent) {
      printf "%.4f\t%d\t%s\n", $max_percent{$symbol}, $max_samples{$symbol}, $symbol;
    }
  }
' "$FLAMEGRAPH_PATH" | sort -nr | head -n "$LIMIT" | awk -F'\t' '{printf "%6.2f%%  %5s samples  %s\n", $1, $2, $3}' || true
