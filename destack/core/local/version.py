from datetime import datetime
from pathlib import Path

from .. import VERSION
from . import console
from .parser import create_cli

cli = create_cli(help="Destack Version management.")


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
    console.print(f"Version: {current_version} -> {new_version}", "bright_yellow")

    # check that version is in all files first
    files_to_update = (
        "pyproject.toml",
        "package.json",
        "Cargo.toml",
        "destack/core/builtin/const.py",
        "destack-py-server/pyproject.toml",
        "destack-ts/package.json",
        "destack-ts-web/package.json",
        "destack-ts-server/package.json",
        "destack-ts-system/package.json",
        "destack-test/package.json",
        "destack-ts/src/language/core/builtin/const.ts",
        "destack-ts-web/src/utils/globals.ts",
    )

    file_texts = {}
    for path in files_to_update:
        file_texts[path] = Path(path).read_text()
        if (
            current_version not in file_texts[path]
            and current_version_semver not in file_texts[path]
        ):
            raise ValueError(f"{current_version} / {current_version_semver} not found in {path}")

    # write version to 'version' file and update all other files
    Path("version").write_text(new_version)
    for path in files_to_update:
        updated_text = (
            file_texts[path]
            .replace(current_version, new_version)
            .replace(current_version_semver, new_version_semver)
        )
        Path(path).write_text(updated_text)
