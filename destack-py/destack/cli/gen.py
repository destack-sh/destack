import structlog
import typer

from destack.utils.oracle import REAL_ORACLE

app = typer.Typer()
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
def gen():
    """Generate all the derived things."""
    from destack.proto.gen import _gen_proto, _gen_proto_schema

    # proto
    start = REAL_ORACLE.time_ns()
    schema_str = _gen_proto_schema()
    _gen_proto(schema_str)
    logger.info("proto.gen", duration=REAL_ORACLE.time_ns() - start)
