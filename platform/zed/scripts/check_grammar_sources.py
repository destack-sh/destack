#!/usr/bin/env python3
"""Validate zed grammar references in extension.toml."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path


PINNED_REV_PATTERN = re.compile(r"^[0-9a-f]{40}$")


@dataclass(frozen=True)
class GrammarReference:
    """A grammar source reference from extension.toml."""

    name: str
    repository: str
    revision: str
    path: str


def run_command(command: list[str], *, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    """Run a process command and capture text output."""

    return subprocess.run(
        command,
        cwd=str(cwd) if cwd is not None else None,
        text=True,
        capture_output=True,
        check=False,
    )


def load_grammar_references(extension_manifest_path: Path) -> list[GrammarReference]:
    """Load grammar references from extension.toml."""

    extension_manifest = tomllib.loads(extension_manifest_path.read_text())
    grammar_table = extension_manifest.get("grammars", {})
    references: list[GrammarReference] = []

    for grammar_name, grammar_value in grammar_table.items():
        if not isinstance(grammar_value, dict):
            continue

        repository = grammar_value.get("repository")
        revision = grammar_value.get("rev")
        path = grammar_value.get("path")
        if not isinstance(repository, str) or not isinstance(revision, str) or not isinstance(path, str):
            continue

        references.append(
            GrammarReference(name=grammar_name, repository=repository, revision=revision, path=path)
        )

    return references


def ensure_local_sources_exist(grammar_reference: GrammarReference, *, repository_root: Path) -> list[str]:
    """Validate local grammar source files for a reference."""

    errors: list[str] = []
    grammar_path = repository_root / grammar_reference.path
    parser_source_path = grammar_path / "src" / "parser.c"

    if not grammar_path.exists():
        errors.append(
            f"`{grammar_reference.name}` local grammar path is missing: `{grammar_reference.path}`"
        )

    if not parser_source_path.exists():
        errors.append(
            f"`{grammar_reference.name}` parser source is missing: `{grammar_reference.path}/src/parser.c`"
        )

    return errors


def ensure_revision_is_available(grammar_reference: GrammarReference) -> list[str]:
    """Ensure the pinned revision object is available locally."""

    verify_result = run_command(["git", "rev-parse", "--verify", "--quiet", f"{grammar_reference.revision}^{{commit}}"])
    if verify_result.returncode == 0:
        return []

    fetch_result = run_command(["git", "fetch", "--depth", "1", grammar_reference.repository, grammar_reference.revision])
    if fetch_result.returncode != 0:
        stderr = fetch_result.stderr.strip() or "<no stderr>"
        return [
            f"`{grammar_reference.name}` failed to fetch revision `{grammar_reference.revision}` "
            f"from `{grammar_reference.repository}`: {stderr}"
        ]

    verify_after_fetch_result = run_command(
        ["git", "rev-parse", "--verify", "--quiet", f"{grammar_reference.revision}^{{commit}}"]
    )
    if verify_after_fetch_result.returncode != 0:
        return [
            f"`{grammar_reference.name}` revision `{grammar_reference.revision}` is not available as a commit"
        ]

    return []


def ensure_revision_paths_exist(grammar_reference: GrammarReference) -> list[str]:
    """Validate grammar path entries exist in the pinned revision tree."""

    errors: list[str] = []
    checks = [
        grammar_reference.path,
        f"{grammar_reference.path}/src",
        f"{grammar_reference.path}/src/parser.c",
    ]

    for check_path in checks:
        cat_file_result = run_command(["git", "cat-file", "-e", f"{grammar_reference.revision}:{check_path}"])
        if cat_file_result.returncode != 0:
            errors.append(
                f"`{grammar_reference.name}` pinned revision `{grammar_reference.revision}` "
                f"is missing `{check_path}`"
            )

    return errors


def is_destack_repository(repository: str) -> bool:
    """Return true when a grammar repository points at destack."""

    return repository in {
        "https://github.com/destack-sh/destack",
        "https://github.com/symbol-industries/destack",
    }


def validate_grammar_reference(
    grammar_reference: GrammarReference,
    *,
    repository_root: Path,
    require_pinned_revisions: bool,
    pin_destack_revision: str | None,
) -> tuple[list[str], list[str]]:
    """Validate one grammar reference and return warnings and errors."""

    warnings: list[str] = []
    errors: list[str] = []

    errors.extend(ensure_local_sources_exist(grammar_reference, repository_root=repository_root))

    effective_revision = grammar_reference.revision
    is_pinned_revision = PINNED_REV_PATTERN.fullmatch(effective_revision) is not None
    if (
        not is_pinned_revision
        and pin_destack_revision is not None
        and is_destack_repository(grammar_reference.repository)
    ):
        effective_revision = pin_destack_revision
        is_pinned_revision = True
        warnings.append(
            f"`{grammar_reference.name}` uses `{grammar_reference.revision}` in source, "
            f"validated against release override `{effective_revision}`"
        )

    effective_reference = GrammarReference(
        name=grammar_reference.name,
        repository=grammar_reference.repository,
        revision=effective_revision,
        path=grammar_reference.path,
    )

    if not is_pinned_revision:
        if require_pinned_revisions:
            errors.append(
                f"`{grammar_reference.name}` revision must be a pinned commit hash in strict mode, "
                f"but got `{grammar_reference.revision}`"
            )
        else:
            warnings.append(
                f"`{grammar_reference.name}` uses non-pinned revision `{grammar_reference.revision}`"
            )
        return warnings, errors

    errors.extend(ensure_revision_is_available(effective_reference))
    if errors:
        return warnings, errors

    errors.extend(ensure_revision_paths_exist(effective_reference))
    return warnings, errors


def parse_arguments() -> argparse.Namespace:
    """Parse command line arguments."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--extension-manifest",
        default="platform/zed/extension.toml",
        help="Path to the zed extension manifest",
    )
    parser.add_argument(
        "--require-pinned",
        action="store_true",
        help="Require grammar revisions to be pinned full commit hashes",
    )
    parser.add_argument(
        "--pin-destack-revision",
        help=(
            "Override non-pinned revisions for destack repository grammar sources with this "
            "commit hash during validation"
        ),
    )
    return parser.parse_args()


def main() -> int:
    """Run grammar reference verification."""

    arguments = parse_arguments()
    if arguments.pin_destack_revision is not None and PINNED_REV_PATTERN.fullmatch(
        arguments.pin_destack_revision
    ) is None:
        print(
            f"error: --pin-destack-revision must be a 40 character commit hash, got `{arguments.pin_destack_revision}`",
            file=sys.stderr,
        )
        return 1

    repository_root = Path(__file__).resolve().parents[3]
    extension_manifest_path = repository_root / arguments.extension_manifest

    if not extension_manifest_path.exists():
        print(f"error: extension manifest not found: {extension_manifest_path}", file=sys.stderr)
        return 1

    grammar_references = load_grammar_references(extension_manifest_path)
    if not grammar_references:
        print("error: no grammar references found in extension manifest", file=sys.stderr)
        return 1

    all_warnings: list[str] = []
    all_errors: list[str] = []

    for grammar_reference in grammar_references:
        warnings, errors = validate_grammar_reference(
            grammar_reference,
            repository_root=repository_root,
            require_pinned_revisions=arguments.require_pinned,
            pin_destack_revision=arguments.pin_destack_revision,
        )
        all_warnings.extend(warnings)
        all_errors.extend(errors)

    for warning in all_warnings:
        print(f"warning: {warning}")

    if all_errors:
        for error in all_errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    print(
        f"validated {len(grammar_references)} zed grammar reference(s) in `{arguments.extension_manifest}`"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
