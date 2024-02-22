import json
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

    # write version to 'const', 'version' and 'package.json'
    Path("bench/language/const.py").write_text(
        Path("bench/language/const.py").read_text().replace(current_version, new_version)
    )
    Path("version").write_text(new_version)
    with open("bench-web/package.json", "r") as f:
        package = json.load(f)
        package["version"] = new_version
    with open("bench-web/package.json", "w") as f:
        json.dump(package, f, indent=2)
