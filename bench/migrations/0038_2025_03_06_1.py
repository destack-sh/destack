# This migration was automatically generated on 2025.03.06. Edit as needed.
import psycopg

ID = 38
VERSION = "2025.03.06.1"
HAS_GLOBAL = True
HAS_REGIONAL = False
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # rename Bench.owner -> Bench.owned_by
    await cur.execute('ALTER TABLE "bench_bench" RENAME COLUMN "owner_id" TO "owned_by_id"')
    await cur.execute('ALTER TABLE "bench_bench" RENAME COLUMN "owner_type" TO "owned_by_type"')
    # Bench.owned_by_ck  is new but equals Bench.owned_by_id
    await cur.execute('ALTER TABLE "bench_bench" ADD COLUMN "owned_by_ck" UUID')
    await cur.execute(
        'UPDATE "bench_bench" SET "owned_by_ck" = "owned_by_id" WHERE "owned_by_id" IS NOT NULL'
    )
    await cur.execute('ALTER TABLE "bench_bench" ALTER COLUMN "owned_by_ck" SET NOT NULL')


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
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
