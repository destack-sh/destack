# This migration was automatically generated on 2025.02.05. Edit as needed.
import psycopg

ID = 5
VERSION = "2025.02.05.3"
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
    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "channel_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "parent_type"')

    # bench_thread
    await cur.execute('ALTER TABLE "bench_thread" ADD COLUMN "scope_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_thread" ADD COLUMN "created_from_id" uuid')
    await cur.execute('ALTER TABLE "bench_thread" ADD COLUMN "created_from_base_ck" uuid')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "channel_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "thread_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "scope_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "scope_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "scope_type" smallint')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "scope_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "forwarded_from_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "forwarded_from_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "forwarded_from_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "forwarded_from_base_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "spawned_thread_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "spawned_thread_base_ck" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
