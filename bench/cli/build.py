from pathlib import Path

import structlog
import typer

from bench.cli.utils import _shell
from bench.proto.build import _build_proto, _build_proto_schema
from bench.sql.build import _build_sql_schema
from bench.utils.dt import monons

app = typer.Typer()
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
def build(no_sql: bool = False, no_proto: bool = False):
    # sql
    if not no_sql:
        start = monons()
        source = _build_sql_schema()
        Path("bench/sql/schema.py").write_text(source)
        _shell("ruff check --fix bench/sql/schema.py")
        _shell("ruff format bench/sql/schema.py")
        logger.info("sql.build", duration=monons() - start)

    # proto
    if not no_proto:
        start = monons()
        schema_str = _build_proto_schema()
        _build_proto(schema_str)
        logger.info("proto.build", duration=monons() - start)
