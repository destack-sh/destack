from datetime import datetime
from pathlib import Path

import structlog
import typer

app = typer.Typer(short_help="version management")
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
def show():
    """
    Show the current version.
    """
    from bench.language.core import VERSION

    print(VERSION)  # noqa: T201


@app.command()
def bump(revision: int | None = typer.Option(None)):
    """
    Bump the CalVer to the current date. Increment revision if the date is the same.
    Format: YYYY.MM.DD.R
    """
    from bench.language.core import VERSION

    current_version = VERSION
    current_version_date = datetime.strptime(current_version[:10], "%Y.%m.%d").date()  # noqa: DTZ007
    current_version_revision = int(current_version[11:])
    today = datetime.today().date()  # noqa: DTZ002
    if revision is None:
        revision = current_version_revision + 1 if current_version_date == today else 0

    new_version = today.strftime("%Y.%m.%d") + "." + str(revision)
    logger.info("version.bump", current_version=current_version, new_version=new_version)

    # write version to 'version', Python files and TS files
    Path("version").write_text(new_version)
    for path in (
        "bench-py/bench/language/core/builtin/const.py",
        "bench-py/bench/pb2/__init__.py",
        "bench-ts/package.json",
        "bench-ts/src/utils/globals.ts",
    ):
        original_text = Path(path).read_text()
        if current_version not in original_text:
            raise ValueError(f"{current_version} not found in {path}")
        updated_text = original_text.replace(current_version, new_version)
        Path(path).write_text(updated_text)
