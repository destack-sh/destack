import structlog
import typer

app = typer.Typer()
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
def generate():
    """Generate all the derived things."""
    from destack.grpc import build_proto, generate_proto_schema
    from destack.language import NODE_TYPES

    # proto
    proto_schema = generate_proto_schema(
        name="symbol.destack",
        unions={"SomeNode": ("node", NODE_TYPES)},
        extras=[],
        message_postfix="Data",
    )
    build_proto(proto_schema)
    logger.info("destack.generate")
