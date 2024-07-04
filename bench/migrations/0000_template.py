# <Header>
import psycopg

ID = "<ID>"
VERSION = "<VERSION>"
HAS_GLOBAL = "<HAS_GLOBAL>"
HAS_LOCAL = "<HAS_LOCAL>"


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass  # <upgrade_global>


async def downgrade_global(cur: psycopg.AsyncCursor):
    pass  # <downgrade_global>


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass  # <upgrade_local>


async def downgrade_local(cur: psycopg.AsyncCursor):
    pass  # <downgrade_local>
