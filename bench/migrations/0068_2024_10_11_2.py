# This migration was automatically generated on 2024.10.11. Edit as needed.
import psycopg

ID = 68
VERSION = "2024.10.11.2"
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
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "base_bench_id"')
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "base_ck"')
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "base_id"')
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "read_type"')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "type" smallint NOT NULL')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "block_id" uuid')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "block_ck" uuid')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "block_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "roots_id" uuid[] NOT NULL')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "roots_ck" uuid[] NOT NULL')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "roots_type" smallint[] NOT NULL')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "roots_bench_id" uuid[] NOT NULL')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "roots_base_ck" uuid[]')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "roots_base_bench_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "aggregation" jsonb')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "ancestor_types" smallint[] NOT NULL')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "descendant_types" smallint[] NOT NULL')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "select" jsonb')
    await cur.execute(
        'ALTER TABLE "bench_query" ADD COLUMN "include_deleted" boolean NOT NULL DEFAULT false'
    )
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "first" integer')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "skip" integer')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
