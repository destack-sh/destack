#!/usr/bin/env python3

from __future__ import annotations

import os
import re
import sys
import tomllib
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
PROJECTS_FILE = REPOSITORY_ROOT / "PROJECTS.toml"

ALLOWED_AREAS = {
    "language",
    "library",
    "service",
    "app",
    "bridge",
    "template",
}

ALLOWED_STATUSES = {
    "Experimental",
    "Alpha",
    "Beta",
    "Stable",
    "Deprecated",
}

AREA_READMES = {
    "language": REPOSITORY_ROOT / "language/README.md",
    "library": REPOSITORY_ROOT / "library/README.md",
    "service": REPOSITORY_ROOT / "service/README.md",
    "app": REPOSITORY_ROOT / "app/README.md",
    "bridge": REPOSITORY_ROOT / "bridge/README.md",
    "template": REPOSITORY_ROOT / "template/README.md",
}


def fail(message: str) -> None:
    """Exit with a validation error."""

    print(message, file=sys.stderr)
    sys.exit(1)


def load_projects() -> list[dict[str, object]]:
    """Load and validate the raw project manifest."""

    with PROJECTS_FILE.open("rb") as file:
        data = tomllib.load(file)

    if data.get("version") != 1:
        fail("PROJECTS.toml must declare version = 1")

    projects = data.get("project")
    if not isinstance(projects, list):
        fail("PROJECTS.toml must define [[project]] entries")

    seen_ids: set[str] = set()

    for project in projects:
        if not isinstance(project, dict):
            fail("PROJECTS.toml project entries must be tables")

        validate_project(project, seen_ids)

    return projects


def validate_project(project: dict[str, object], seen_ids: set[str]) -> None:
    """Validate one project entry."""

    required_keys = {
        "id",
        "area",
        "name",
        "path",
        "readme",
        "status",
        "languages",
        "license",
        "tags",
        "summary",
    }

    missing_keys = sorted(required_keys - set(project.keys()))
    if missing_keys:
        fail(f"PROJECTS.toml entry is missing required keys: {', '.join(missing_keys)}")

    project_id = require_non_empty_string(project, "id")
    if project_id in seen_ids:
        fail(f"PROJECTS.toml project id is duplicated: {project_id}")
    seen_ids.add(project_id)

    area = require_non_empty_string(project, "area")
    if area not in ALLOWED_AREAS:
        fail(f"PROJECTS.toml project '{project_id}' has invalid area: {area}")

    status = require_non_empty_string(project, "status")
    if status not in ALLOWED_STATUSES:
        fail(f"PROJECTS.toml project '{project_id}' has invalid status: {status}")

    require_non_empty_string(project, "name")
    require_non_empty_string(project, "license")
    require_non_empty_string(project, "summary")

    languages = project["languages"]
    if not isinstance(languages, list) or not languages:
        fail(f"PROJECTS.toml project '{project_id}' must define at least one language")
    if len(languages) > 2:
        fail(f"PROJECTS.toml project '{project_id}' must define at most two languages")
    if not all(isinstance(language, str) and language for language in languages):
        fail(f"PROJECTS.toml project '{project_id}' has invalid languages")

    tags = project["tags"]
    if not isinstance(tags, list) or not tags:
        fail(f"PROJECTS.toml project '{project_id}' must define at least one tag")
    if not all(isinstance(tag, str) and tag for tag in tags):
        fail(f"PROJECTS.toml project '{project_id}' has invalid tags")

    project_path = REPOSITORY_ROOT / require_non_empty_string(project, "path")
    if not project_path.exists():
        fail(f"PROJECTS.toml project '{project_id}' path does not exist: {project_path}")

    readme_path = REPOSITORY_ROOT / require_non_empty_string(project, "readme")
    if not readme_path.is_file():
        fail(f"PROJECTS.toml project '{project_id}' readme does not exist: {readme_path}")


def require_non_empty_string(project: dict[str, object], key: str) -> str:
    """Read one required string field."""

    value = project.get(key)
    if not isinstance(value, str) or not value:
        project_id = project.get("id", "<unknown>")
        fail(f"PROJECTS.toml project '{project_id}' field '{key}' must be a non-empty string")
    return value


def expected_rows(projects: list[dict[str, object]], area: str) -> list[str]:
    """Build the expected markdown rows for one area."""

    area_readme = AREA_READMES[area]
    area_directory = area_readme.parent
    rows: list[str] = []

    for project in projects:
        if project["area"] != area:
            continue

        name = project["name"]
        status = project["status"]
        summary = project["summary"]
        readme_path = REPOSITORY_ROOT / str(project["readme"])
        relative_readme = os.path.relpath(readme_path, area_directory).replace(os.sep, "/")
        rows.append(f"| [`{name}`]({relative_readme}) | {status} | {summary} |")

    return rows


def extract_project_rows(path: Path) -> list[str]:
    """Extract the project table rows from one area readme."""

    lines = path.read_text().splitlines()

    try:
        heading_index = lines.index("## Projects")
    except ValueError:
        fail(f"{path.relative_to(REPOSITORY_ROOT)} is missing a '## Projects' section")

    header_index = -1
    for index in range(heading_index + 1, len(lines)):
        if lines[index].strip() == "| Project | Status | Summary |":
            header_index = index
            break

    if header_index == -1:
        fail(f"{path.relative_to(REPOSITORY_ROOT)} is missing the project inventory table header")

    separator_index = header_index + 1
    if separator_index >= len(lines) or not lines[separator_index].strip().startswith("|---"):
        fail(f"{path.relative_to(REPOSITORY_ROOT)} is missing the project inventory table separator")

    rows: list[str] = []
    for line in lines[separator_index + 1 :]:
        if not line.startswith("|"):
            break
        rows.append(line.rstrip())

    return rows


def validate_area_tables(projects: list[dict[str, object]]) -> None:
    """Validate every area readme against the manifest."""

    for area, readme_path in AREA_READMES.items():
        actual_rows = extract_project_rows(readme_path)
        wanted_rows = expected_rows(projects, area)

        if actual_rows != wanted_rows:
            actual = "\n".join(actual_rows)
            wanted = "\n".join(wanted_rows)
            fail(
                f"{readme_path.relative_to(REPOSITORY_ROOT)} project table is out of sync with PROJECTS.toml\n"
                f"expected:\n{wanted}\n\nactual:\n{actual}"
            )


def main() -> None:
    """Run the project docs validation."""

    projects = load_projects()
    validate_area_tables(projects)


if __name__ == "__main__":
    main()
