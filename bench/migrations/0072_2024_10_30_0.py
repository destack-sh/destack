# This migration was automatically generated on 2024.10.30. Edit as needed.
import psycopg

ID = 72
VERSION = "2024.10.30.0"
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
    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "combinator"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "run_options"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "code"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "options"')

    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "delay"')
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "line"')
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "mapping"')
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "modulation"')
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "repeat"')
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "size"')
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "source_port"')
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "target_port"')
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "filter"')
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "filter" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
