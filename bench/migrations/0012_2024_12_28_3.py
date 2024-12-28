# This migration was automatically generated on 2024.12.28. Edit as needed.
import psycopg

ID = 12
VERSION = "2024.12.28.3"
HAS_GLOBAL = False
HAS_REGIONAL = True
HAS_LOCAL = False


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
    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "scaler_id" uuid')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "scaler_bench_id" uuid')

    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "scaler_id" uuid')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "scaler_bench_id" uuid')

    # bench_file
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "scaler_id" uuid')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "scaler_bench_id" uuid')

    # bench_stream
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "scaler_id" uuid')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "scaler_bench_id" uuid')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "scaler_id" uuid')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "scaler_bench_id" uuid')


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
