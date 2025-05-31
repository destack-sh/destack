from pathlib import Path

import structlog
import typer

from bench.utils.oracle import REAL_ORACLE

app = typer.Typer()
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
def gen():
    """Generate all the derived things."""
    from bench.proto.gen import _gen_proto, _gen_proto_schema
    from bench.sql.gen import _gen_sql_schema

    from .utils import run_shell_sync

    # sql
    start = REAL_ORACLE.time_ns()
    source = _gen_sql_schema()
    Path("bench-py/bench/sql/schema.py").write_text(source)
    run_shell_sync("ruff check --fix bench-py/bench/sql/schema.py")
    run_shell_sync("ruff format bench-py/bench/sql/schema.py")
    logger.info("sql.gen", duration=REAL_ORACLE.time_ns() - start)

    # proto
    start = REAL_ORACLE.time_ns()
    schema_str = _gen_proto_schema()
    _gen_proto(schema_str)
    logger.info("proto.gen", duration=REAL_ORACLE.time_ns() - start)
