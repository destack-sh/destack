# This migration was automatically generated on 2024.10.01. Edit as needed.
import psycopg

ID = 61
VERSION = "2024.10.01.1"
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
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "halted_at"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "halted_epoch"')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "active_duration" interval')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
