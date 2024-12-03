from pathlib import Path

import structlog
import typer

from bench.utils.oracle import REAL_ORACLE

app = typer.Typer()
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
async def build(no_sql: bool = False, no_proto: bool = False, no_builtin: bool = False):
    from bench.cli.utils import run_shell_sync
    from bench.language.builtin import BUILTINS
    from bench.proto.build import _build_proto, _build_proto_schema
    from bench.runtime.base import BENCH_QUERY, PACKAGE_QUERY
    from bench.sql.build import _build_sql_schema

    # sql
    if not no_sql:
        start = REAL_ORACLE.time_ns()
        source = _build_sql_schema()
        Path("bench/sql/schema.py").write_text(source)
        run_shell_sync("ruff check --fix bench/sql/schema.py")
        run_shell_sync("ruff format bench/sql/schema.py")
        logger.info("sql.build", duration=REAL_ORACLE.time_ns() - start)

    # proto
    if not no_proto:
        start = REAL_ORACLE.time_ns()
        schema_str = _build_proto_schema()
        _build_proto(schema_str)
        logger.info("proto.build", duration=REAL_ORACLE.time_ns() - start)

    # builtins
    if not no_builtin:
        # nocheckin: this probably doesn't belong here but in the migrate path..?
        bench = await BENCH_QUERY.get(slug="bench")
        package = await PACKAGE_QUERY.get(bench.main_package_ptr)
        await patch_graph(package, BUILTINS)
