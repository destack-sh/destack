# This migration was automatically generated on 2024.11.29. Edit as needed.
import psycopg

ID = 7
VERSION = "2024.11.29.0"
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
    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_session
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_interrupt
    await cur.execute('ALTER TABLE "bench_interrupt" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
