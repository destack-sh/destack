# This migration was automatically generated on 2025.01.15. Edit as needed.
import psycopg

ID = 31
VERSION = "2025.01.14.5"
HAS_GLOBAL = True
HAS_REGIONAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_bench
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "policies"')


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
    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "policies"')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "delegated_policies"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "policies"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "run_options"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
