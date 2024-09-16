# This migration was automatically generated on 2024.09.16. Edit as needed.
import psycopg

ID = 47
VERSION = "2024.09.16.0"
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
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "halted_on_run_base_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "halted_on_run_id"')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "cached_duration" real')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
