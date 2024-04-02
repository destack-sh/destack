from datetime import datetime
from pathlib import Path

import structlog
import typer

from bench.language.const import VERSION

app = typer.Typer(short_help="version management")
logger = structlog.get_logger(__name__)


@app.command()
def bump(revision: int = typer.Option(None)):
    """
    Bump the CalVer to the current date. Increment revision if the date is the same.
    Format is YYYY.MM.DD.R
    """
    current_version = VERSION
    current_version_date = datetime.strptime(current_version[:10], "%Y.%m.%d").date()
    current_version_revision = int(current_version[11:])
    today = datetime.today().date()
    if revision is None:
        revision = current_version_revision + 1 if current_version_date == today else 0

    new_version = today.strftime("%Y.%m.%d") + "." + str(revision)
    logger.info("version.bump", current_version=current_version, new_version=new_version)

    # write version to 'version', Python files and TS files
    for path in (
        "version",
        "bench/language/const.py",
        "bench/sql/schema.py",
        "bench/proto/wire.py",
        "bench-web/package.json",
        "bench-web/src/utils/globals.ts",
    ):
        Path(path).write_text(Path(path).read_text().replace(current_version, new_version))
