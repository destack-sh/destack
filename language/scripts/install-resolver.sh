#!/usr/bin/env bash
set -euo pipefail

fixtures_directory="test/fixtures/resolver"
required_fixture_sources=(
	"$fixtures_directory/enhanced_resolve/test/fixtures/node_modules/m1/a.js"
	"$fixtures_directory/enhanced_resolve/test/fixtures/multiple_modules/node_modules/m1/a.js"
	"$fixtures_directory/bench-tsconfig-paths/README.md"
	"$fixtures_directory/tsconfck/package.json"
)

for fixture_source_path in "${required_fixture_sources[@]}"; do
	if [ ! -f "$fixture_source_path" ]; then
		echo "missing vendored resolver fixture source: $fixture_source_path"
		echo "restore resolver fixtures from git before running install-resolver"
		exit 1
	fi
done

if [ -f "$fixtures_directory/pnpm/package.json" ]; then
	(cd "$fixtures_directory/pnpm" && CI=1 pnpm install --ignore-workspace)
fi

if [ -f "$fixtures_directory/pnpm-workspace/package.json" ]; then
	(cd "$fixtures_directory/pnpm-workspace" && CI=1 pnpm install)
fi

for directory in "$fixtures_directory/yarn" "$fixtures_directory/global-pnp" "$fixtures_directory/pnp"; do
	if [ -f "$directory/package.json" ]; then
		(cd "$directory" && CI=1 corepack yarn install)
	fi
done
