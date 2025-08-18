import os
from datetime import datetime
from pathlib import Path

from .. import VERSION
from . import _console
from ._parser import create_cli

cli = create_cli(
    "version",
    aliases=["v", "ver"],
    help="Mark new versions.",
)


def _to_semver(calver: str) -> str:
    """
    Convert CalVer to SemVer.
    Strip leading zeros, replace last . with -.
    e.g. 2025.08.06.1 -> 2025.8.6-1
    """
    parts = calver.split(".")
    if len(parts) != 4:
        raise ValueError(f"invalid CalVer format: {calver}")
    year = str(int(parts[0]))
    month = str(int(parts[1]))
    day = str(int(parts[2]))
    revision = parts[3]
    return f"{year}.{month}.{day}-{revision}"


@cli.command()
def bump(revision: int | None = None):
    """
    Bump the CalVer. Increment revision if needed (format: YYYY.MM.DD.R).
    """

    current_version = VERSION
    current_version_date = datetime.strptime(current_version[:10], "%Y.%m.%d").date()  # noqa: DTZ007
    current_version_revision = int(current_version[11:])
    current_version_semver = _to_semver(current_version)

    today = datetime.today().date()  # noqa: DTZ002
    if revision is None:
        revision = current_version_revision + 1 if current_version_date == today else 0

    new_version = today.strftime("%Y.%m.%d") + "." + str(revision)
    new_version_semver = _to_semver(new_version)
    _console.print(f"Version: {current_version} -> {new_version}", "bright_yellow")

    # check that version is in all files first
    files_to_update = (
        "pyproject.toml",
        "package.json",
        "Cargo.toml",
        "destack-ds/destack/core/builtin/_const.py",
        "destack-ds/pyproject.toml",
        "destack-py/pyproject.toml",
        "destack-ts/destack/package.json",
        "destack-ts/destack_web/package.json",
    )

    file_texts = {}
    for raw_path in files_to_update:
        path = Path("../" + raw_path)
        if not path.exists():
            raise ValueError(f"{path} not found from {os.getcwd()}")
        file_texts[raw_path] = path.read_text()
        if (
            current_version not in file_texts[raw_path]
            and current_version_semver not in file_texts[raw_path]
        ):
            raise ValueError(
                f"{current_version} / {current_version_semver} not found in {raw_path}"
            )

    # write version to 'version' file and update all other files
    Path("version").write_text(new_version)
    for raw_path in files_to_update:
        path = Path("../" + raw_path)
        updated_text = (
            file_texts[raw_path]
            .replace(current_version, new_version)
            .replace(current_version_semver, new_version_semver)
        )
        Path(path).write_text(updated_text)
