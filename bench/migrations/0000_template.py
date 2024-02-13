# <Header>
import psycopg

ID = "<ID>"
VERSION = "<VERSION>"
HAS_GLOBAL = "<HAS_GLOBAL>"
HAS_LOCAL = "<HAS_LOCAL>"


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass  # <upgrade_global>


async def downgrade_global(cur: psycopg.AsyncCursor):
    pass  # <downgrade_global>


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass  # <upgrade_local>


async def downgrade_local(cur: psycopg.AsyncCursor):
    pass  # <downgrade_local>
