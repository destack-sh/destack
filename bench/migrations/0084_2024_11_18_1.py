# This migration was automatically generated on 2024.11.18. Edit as needed.
import psycopg

ID = 84
VERSION = "2024.11.18.1"
HAS_GLOBAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "paused_at" timestamp')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "intermediates_packed" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
