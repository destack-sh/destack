# This migration was automatically generated on 2024.08.08. Edit as needed.
import psycopg

ID = 35
VERSION = "2024.08.08.4"
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
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "current_status"')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "killed_at" timestamp')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
