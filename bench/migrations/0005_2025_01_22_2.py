# This migration was automatically generated on 2025.01.22. Edit as needed.
import psycopg

ID = 5
VERSION = "2025.01.22.2"
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
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "caller_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "caller_base_ck" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
