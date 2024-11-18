# This migration was automatically generated on 2024.11.18. Edit as needed.
import psycopg

ID = 83
VERSION = "2024.11.18.0"
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
    # bench_interrupt
    await cur.execute('ALTER TABLE "bench_interrupt" DROP COLUMN "opened_at"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "resumed_at" timestamp')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
