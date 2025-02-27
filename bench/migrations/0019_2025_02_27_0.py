# This migration was automatically generated on 2025.02.27. Edit as needed.
import psycopg

ID = 19
VERSION = "2025.02.27.0"
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
    await cur.execute('ALTER TABLE "bench_action" ADD COLUMN "category" smallint')
    await cur.execute('ALTER TABLE "bench_action" ADD COLUMN "tool_type" smallint')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
