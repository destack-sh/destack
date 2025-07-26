from datetime import datetime
from pathlib import Path

import typer

from destack.utils.log import get_logger

app = typer.Typer(short_help="version management")
logger = get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
def show():
    """
    Show the current version.
    """
    from destack.language.core import VERSION

    print(VERSION)  # noqa: T201


@app.command()
def bump(revision: int | None = typer.Option(None)):
    """
    Bump the CalVer to the current date. Increment revision if the date is the same.
    Format: YYYY.MM.DD.R
    """
    from destack.language.core import VERSION

    current_version = VERSION
    current_version_date = datetime.strptime(current_version[:10], "%Y.%m.%d").date()  # noqa: DTZ007
    current_version_revision = int(current_version[11:])
    today = datetime.today().date()  # noqa: DTZ002
    if revision is None:
        revision = current_version_revision + 1 if current_version_date == today else 0

    new_version = today.strftime("%Y.%m.%d") + "." + str(revision)
    logger.info("version.bump", current_version=current_version, new_version=new_version)

    # check that version is in all files first
    files_to_update = (
        "pyproject.toml",
        "package.json",
        "destack-py/destack/language/core/builtin/const.py",
        "destack-ts/package.json",
        "destack-ts-web/package.json",
        "destack-ts-system/package.json",
        "destack-ts-test/package.json",
        "destack-ts/src/language/core/builtin/const.ts",
        "destack-ts-web/src/utils/globals.ts",
    )

    for path in files_to_update:
        original_text = Path(path).read_text()
        if current_version not in original_text:
            raise ValueError(f"{current_version} not found in {path}")

    # write version to 'version' file and update all other files
    Path("version").write_text(new_version)
    for path in files_to_update:
        original_text = Path(path).read_text()
        updated_text = original_text.replace(current_version, new_version)
        Path(path).write_text(updated_text)
