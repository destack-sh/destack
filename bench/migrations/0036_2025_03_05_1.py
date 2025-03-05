# This migration was automatically generated on 2025.03.05. Edit as needed.
import psycopg

ID = 36
VERSION = "2025.03.05.1"
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
    await cur.execute('ALTER TABLE "bench_implementation" RENAME TO "bench_kit"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
