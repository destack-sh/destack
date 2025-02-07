# This migration was automatically generated on 2025.02.07. Edit as needed.
import psycopg

ID = 6
VERSION = "2025.02.07.0"
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
    # bench_page
    await cur.execute('ALTER TABLE "bench_page" DROP COLUMN "order_key"')

    # bench_channel
    await cur.execute('ALTER TABLE "bench_channel" DROP COLUMN "order_key"')

    # bench_page
    await cur.execute('ALTER TABLE "bench_page" ADD COLUMN "block_id" uuid')
    await cur.execute('ALTER TABLE "bench_page" ADD COLUMN "block_ck" uuid')
    await cur.execute('ALTER TABLE "bench_page" ADD COLUMN "block_bench_id" uuid')

    # bench_channel
    await cur.execute('ALTER TABLE "bench_channel" ADD COLUMN "block_id" uuid')
    await cur.execute('ALTER TABLE "bench_channel" ADD COLUMN "block_ck" uuid')
    await cur.execute('ALTER TABLE "bench_channel" ADD COLUMN "block_bench_id" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
