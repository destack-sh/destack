# This migration was automatically generated on 2025.02.25. Edit as needed.
import psycopg

ID = 17
VERSION = "2025.02.25.2"
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
    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "base_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "base_base_ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "base_bench_id"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "base_ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "base_id"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "base_type"')
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "channel_id" uuid')
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "channel_ck" uuid')
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "channel_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "thread_id" uuid')
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "thread_bench_id" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
