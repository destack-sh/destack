from datetime import datetime
from pathlib import Path

from destack.cli.parser import create_cli
from destack.language.core import VERSION
from destack.utils.log import get_logger

cli = create_cli(help="Destack Version management.")
logger = get_logger(__name__)


@cli.command()
def bump(revision: int | None = None):
    """
    Bump the CalVer. Increment revision if needed (format: YYYY.MM.DD.R).
    """

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
        "destack-py-server/pyproject.toml",
        "destack-ts/package.json",
        "destack-ts-web/package.json",
        "destack-ts-server/package.json",
        "destack-ts-system/package.json",
        "destack-ts-test/package.json",
        "destack-ts/src/language/core/builtin/const.ts",
        "destack-ts-web/src/utils/globals.ts",
    )

    file_texts = {}
    for path in files_to_update:
        file_texts[path] = Path(path).read_text()
        if current_version not in file_texts[path]:
            raise ValueError(f"{current_version} not found in {path}")

    # write version to 'version' file and update all other files
    Path("version").write_text(new_version)
    for path in files_to_update:
        updated_text = file_texts[path].replace(current_version, new_version)
        Path(path).write_text(updated_text)
