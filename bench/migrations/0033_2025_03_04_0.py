# This migration was automatically generated on 2025.03.04. Edit as needed.
import psycopg

ID = 33
VERSION = "2025.03.04.0"
HAS_GLOBAL = False
HAS_REGIONAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    pass


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_action
    await cur.execute('ALTER TABLE "bench_action" RENAME COLUMN "tool_selection" TO "selection"')

    # bench_flow
    await cur.execute('ALTER TABLE "bench_flow" RENAME COLUMN "tool_selection" TO "selection"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
