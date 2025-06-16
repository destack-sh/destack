# <Header>
import asyncpg

ID = "<ID>"
VERSION = "<VERSION>"
HAS_GLOBAL = "<HAS_GLOBAL>"
HAS_SPATIAL = "<HAS_SPATIAL>"


#
# Global DB
#


async def upgrade_global(conn: asyncpg.Connection):
    pass  # <upgrade_global>


async def downgrade_global(conn: asyncpg.Connection):
    pass  # <downgrade_global>


#
# Main DB
#


async def upgrade_main(conn: asyncpg.Connection):
    pass  # <upgrade_main>


async def downgrade_main(conn: asyncpg.Connection):
    pass  # <downgrade_main>
