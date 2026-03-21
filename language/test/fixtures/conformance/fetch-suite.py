#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import shutil
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


def resolve_relative_path(base: Path, value: str) -> Path:
    """Resolve one relative path against one base directory."""

    return (base / value).resolve()


def ensure_test_path(test_directory: Path, value: str) -> Path:
    """Resolve one test path under the suite test directory."""

    path = (test_directory / value).resolve()
    if not path.is_relative_to(test_directory.resolve()):
        raise ValueError(f"test path escapes the suite: {value}")

    return path


def materialize_source_entry(entry: dict, suite_directory: Path, test_directory: Path) -> None:
    """Copy one source entry into the suite test directory."""

    source_root_value = entry.get("root")
    if not isinstance(source_root_value, str) or not source_root_value:
        raise ValueError("source entries must define a non-empty root")

    source_root = resolve_relative_path(suite_directory, source_root_value)

    for file_entry in entry.get("files", []):
        source_value = file_entry.get("source")
        target_value = file_entry.get("target", source_value)
        if not isinstance(source_value, str) or not source_value:
            raise ValueError("source file entries must define a non-empty source path")
        if not isinstance(target_value, str) or not target_value:
            raise ValueError("source file entries must define a non-empty target path")

        source_path = resolve_relative_path(source_root, source_value)
        target_path = ensure_test_path(test_directory, target_value)

        target_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source_path, target_path)


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

    test_directory = suite_directory / "tests"
    fetch_entries = load_fetch_entries(suite_directory)

    test_directory.mkdir(parents=True, exist_ok=True)

    for entry in fetch_entries:
        entry_kind = entry.get("kind")
        if entry_kind not in VALID_ENTRY_KINDS:
            raise ValueError(f"unsupported entry kind: {entry_kind}")

        # copy imported source files into the checked in test tree
        if entry_kind == "source":
            materialize_source_entry(entry, suite_directory, test_directory)

        # validate locally translated or manual tests
        else:
            validate_local_entry(entry, test_directory)


def repo_root_for_suite(suite_directory: Path) -> Path:
    """Return the repository root for one suite directory."""

    return suite_directory.parents[5]


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
    argument_parser.add_argument("suite_directory")
    argument_parser.add_argument(
        "--diff",
        action="store_true",
        help="print diffs between source and translated test files",
    )
    arguments = argument_parser.parse_args(argv[1:])

    suite_directory = Path(arguments.suite_directory).resolve()
    fetch_suite(suite_directory)

    if arguments.diff:
        print_translation_diffs(suite_directory)

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
