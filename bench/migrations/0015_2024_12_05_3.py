# This migration was automatically generated on 2024.12.05. Edit as needed.
import psycopg

ID = 15
VERSION = "2024.12.05.3"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "killed_at"')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "stopped_at" timestamp')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "killed_at"')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "stopped_at" timestamp')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
