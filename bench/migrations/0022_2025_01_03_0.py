# This migration was automatically generated on 2025.01.03. Edit as needed.
import psycopg

ID = 22
VERSION = "2025.01.03.0"
HAS_GLOBAL = False
HAS_REGIONAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    pass


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_trigger
    await cur.execute('DROP TABLE "bench_trigger"')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "trigger_bench_id"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "trigger_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "trigger_id"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
