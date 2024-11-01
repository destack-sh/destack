# This migration was automatically generated on 2024.11.01. Edit as needed.
import psycopg

ID = 73
VERSION = "2024.11.01.1"
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
    await cur.execute('TRUNCATE "bench_run"')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "pipe_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "pipe_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "pipe_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "options" jsonb NOT NULL')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
