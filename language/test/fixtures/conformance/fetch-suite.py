#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import sys
import subprocess
from pathlib import Path


VALID_ENTRY_KINDS = {"source", "translated", "manual"}
SUITE_JSON_FILE_NAME = "suite.json"


def load_json(path: Path) -> dict:
    """Load one json file."""

    return json.loads(path.read_text())


def load_fetch_entries(suite_directory: Path) -> list[dict]:
    """Load the fetch entries from one suite manifest."""

    suite_path = suite_directory / SUITE_JSON_FILE_NAME
    suite_data = load_json(suite_path)
    fetch_data = suite_data.get("fetch", {})
    if not isinstance(fetch_data, dict):
        raise ValueError(f"invalid fetch manifest in {suite_path}")

    entries = fetch_data.get("entries", [])
    if not isinstance(entries, list):
        raise ValueError(f"invalid fetch entries in {suite_path}")

    return entries


def discover_suite_directories(path: Path) -> list[Path]:
    """Discover suite directories from one suite or domain path."""

    suite_path = path / SUITE_JSON_FILE_NAME
    if suite_path.is_file():
        return [path]

    suite_directories = sorted(
        suite_path.parent for suite_path in path.rglob(SUITE_JSON_FILE_NAME)
    )
    if suite_directories:
        return suite_directories

    raise FileNotFoundError(f"no suite.json found under {path}")


def resolve_relative_path(base: Path, value: str) -> Path:
    """Resolve one relative path against one base directory."""

    return (base / value).resolve()


def fetch_source_entries(suite_directory: Path) -> None:
    """Fetch the source entries for one suite through the shared shell helper."""

    helper = resolve_relative_path(suite_directory, "../../fetch-origin.sh")
    result = subprocess.run([str(helper), str(suite_directory)], check=False)
    if result.returncode != 0:
        raise SystemExit(result.returncode)


def ensure_test_path(test_directory: Path, value: str) -> Path:
    """Resolve one test path under the suite test directory."""

    path = (test_directory / value).resolve()
    if not path.is_relative_to(test_directory.resolve()):
        raise ValueError(f"test path escapes the suite: {value}")

    return path


def validate_local_entry(entry: dict, test_directory: Path) -> None:
    """Validate one translated or manual test entry."""

    for file_entry in entry.get("files", []):
        path_value = file_entry.get("path")
        if not isinstance(path_value, str) or not path_value:
            raise ValueError("translated and manual file entries must define a non-empty path")

        test_path = ensure_test_path(test_directory, path_value)
        if not test_path.is_file():
            raise FileNotFoundError(f"missing local test file: {test_path}")

        source_value = file_entry.get("source")
        if isinstance(source_value, str) and source_value:
            source_path = ensure_test_path(test_directory, source_value)
            if not source_path.is_file():
                raise FileNotFoundError(f"missing source test file: {source_path}")


def fetch_suite(suite_directory: Path) -> None:
    """Fetch one suite from its suite metadata."""

    fetch_entries = load_fetch_entries(suite_directory)
    if not fetch_entries:
        return

    test_directory = suite_directory / "tests"
    if any(entry.get("kind") == "source" for entry in fetch_entries):
        fetch_source_entries(suite_directory)
    else:
        test_directory.mkdir(parents=True, exist_ok=True)

    for entry in fetch_entries:
        entry_kind = entry.get("kind")
        if entry_kind not in VALID_ENTRY_KINDS:
            raise ValueError(f"unsupported entry kind: {entry_kind}")

        # validate locally translated or manual tests
        if entry_kind != "source":
            validate_local_entry(entry, test_directory)


def repo_root_for_suite(suite_directory: Path) -> Path:
    """Return the repository root for one suite directory."""

    for parent in suite_directory.parents:
        if (parent / "Cargo.toml").is_file():
            return parent

    raise FileNotFoundError(f"failed to find repository root for {suite_directory}")


def print_translation_diffs(suite_directory: Path) -> None:
    """Print diffs between source and translated test files."""

    repo_root = repo_root_for_suite(suite_directory)
    command = [
        "cargo",
        "run",
        "-p",
        "destack_test",
        "--bin",
        "print_conformance_translation_diffs",
        "--",
        "--suite-dir",
        str(suite_directory),
    ]

    result = subprocess.run(command, cwd=repo_root, check=False)
    if result.returncode != 0:
        raise SystemExit(result.returncode)


def main(argv: list[str]) -> int:
    """Run the suite materializer."""

    argument_parser = argparse.ArgumentParser()
    argument_parser.add_argument("paths", nargs="+")
    argument_parser.add_argument(
        "--diff",
        action="store_true",
        help="print diffs between source and translated test files",
    )
    arguments = argument_parser.parse_args(argv[1:])

    for path_value in arguments.paths:
        path = Path(path_value).resolve()
        suite_directories = discover_suite_directories(path)

        for suite_directory in suite_directories:
            fetch_suite(suite_directory)

            if arguments.diff:
                print_translation_diffs(suite_directory)

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
