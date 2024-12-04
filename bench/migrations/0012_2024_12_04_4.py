# This migration was automatically generated on 2024.12.04. Edit as needed.
import psycopg

ID = 12
VERSION = "2024.12.04.4"
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
    # bench_query
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "block_bench_id"')
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "block_ck"')
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "block_id"')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "base_block_id" uuid')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "base_block_ck" uuid')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "base_block_bench_id" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
