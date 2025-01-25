# This migration was automatically generated on 2025.01.25. Edit as needed.
import psycopg

ID = 8
VERSION = "2025.01.25.0"
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
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "code" jsonb')

    # bench_run_span
    await cur.execute('ALTER TABLE "bench_run_span" ADD COLUMN "code" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
