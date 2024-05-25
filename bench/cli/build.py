from pathlib import Path

import structlog
import typer

from bench.proto.build import _build_proto, _build_proto_schema
from bench.sql.build import _build_sql_schema
from bench.utils.dt import monotime

app = typer.Typer()
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
def build():
    # sql
    start = monotime()
    source = _build_sql_schema()
    Path("bench/sql/schema.py").write_text(source)
    logger.info("sql.build", duration=monotime() - start)

    # proto
    start = monotime()
    schema_str = _build_proto_schema()
    _build_proto(schema_str)
    logger.info("proto.build", duration=monotime() - start)
