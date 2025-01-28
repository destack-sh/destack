# This migration was automatically generated on 2025.01.28. Edit as needed.
import psycopg

ID = 10
VERSION = "2025.01.28.0"
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
    # bench_package
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "ck" uuid')
    await cur.execute('UPDATE "bench_package" SET "ck" = id')
    await cur.execute('ALTER TABLE "bench_package" ALTER COLUMN "ck" SET NOT NULL')
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "package_id" uuid')
    await cur.execute('UPDATE "bench_package" SET "package_id" = id')
    await cur.execute('ALTER TABLE "bench_package" ALTER COLUMN "package_id" SET NOT NULL')
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "package_ck" uuid')
    await cur.execute('UPDATE "bench_package" SET "package_ck" = ck')
    await cur.execute('ALTER TABLE "bench_package" ALTER COLUMN "package_ck" SET NOT NULL')
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "template_at" timestamp')
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "mode" smallint NOT NULL DEFAULT 2')
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "computed_values" jsonb[]')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
