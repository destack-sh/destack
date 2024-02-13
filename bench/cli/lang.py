import structlog
import typer

from bench.cli.utils import _async_to_sync_blocking, _check_is_consistent
from bench.language import Bench
from bench.language.const import NODE_TYPES
from bench.language.node import CHILD_NODE_TYPES
from bench.system.utils import global_session

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command(help="check whether the current Bench state is properly migrated")
@_async_to_sync_blocking
async def check(check_db: bool = False):
    await _check_is_consistent(check_db=check_db)


@app.command(help="IPython shell with a global or bench-local session")
@_async_to_sync_blocking
async def shell(bench: str = None):
    """Open a Session shell."""
    if bench is not None:
        async with global_session(read_only=True):
            bench: Bench = await Bench.get(slug=bench)
    logger.info("lang.shell", bench=bench)
    raise NotImplementedError("TODO @Dev: shell.session")


@app.command(help="Plots the node type ancestry graph.")
@_async_to_sync_blocking
async def plot():
    import networkx as nx
    import matplotlib.pyplot as plt

    g = nx.DiGraph()
    for node_type in NODE_TYPES:
        g.add_node(node_type.bench_name)

    for node_type in NODE_TYPES:
        for child_type in CHILD_NODE_TYPES[node_type]:
            g.add_edge(node_type.bench_name, child_type.bench_name)

    # plot it
    pos = nx.nx_agraph.graphviz_layout(g, prog="dot")
    nx.draw_networkx_nodes(g, pos, node_size=700)
    nx.draw_networkx_edges(g, pos, width=2)
    plt.axis("off")
    plt.show()
