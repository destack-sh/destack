# <Header>
import asyncpg

ID = "<ID>"
VERSION = "<VERSION>"
HAS_GLOBAL = "<HAS_GLOBAL>"
HAS_REGIONAL = "<HAS_REGIONAL>"
HAS_LOCAL = "<HAS_LOCAL>"


#
# Global DB
#


async def upgrade_global(cur: asyncpg.Connection):
    pass  # <upgrade_global>


async def downgrade_global(cur: asyncpg.Connection):
    pass  # <downgrade_global>


#
# Regional DB
#


async def upgrade_regional(cur: asyncpg.Connection):
    pass  # <upgrade_regional>


async def downgrade_regional(cur: asyncpg.Connection):
    pass  # <downgrade_regional>


#
# Local DB
#


async def upgrade_local(cur: asyncpg.Connection):
    pass  # <upgrade_local>


async def downgrade_local(cur: asyncpg.Connection):
    pass  # <downgrade_local>
