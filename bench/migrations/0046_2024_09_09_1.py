# This migration was automatically generated on 2024.09.09. Edit as needed.
import psycopg

ID = 46
VERSION = "2024.09.09.1"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "restarted_at" timestamp')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "is_paused"')

    # bench_package
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "is_paused"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
