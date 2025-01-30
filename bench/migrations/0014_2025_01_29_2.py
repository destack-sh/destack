# This migration was automatically generated on 2025.01.29. Edit as needed.
import psycopg

ID = 14
VERSION = "2025.01.29.2"
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
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "origin_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "root_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "root_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "nodes_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "nodes_ck" uuid[]')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "nodes_type" smallint[]')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "nodes_bench_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "nodes_base_ck" uuid[]')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "nodes_base_bench_id" uuid[]')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
