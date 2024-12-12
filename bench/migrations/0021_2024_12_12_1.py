# This migration was automatically generated on 2024.12.12. Edit as needed.
import psycopg

ID = 21
VERSION = "2024.12.12.1"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_user
    await cur.execute('ALTER TABLE "bench_user" ADD COLUMN "region" smallint NOT NULL DEFAULT 1002')
    await cur.execute('ALTER TABLE "bench_user" ALTER COLUMN "region" DROP DEFAULT')

    # bench_organization
    await cur.execute(
        'ALTER TABLE "bench_organization" ADD COLUMN "region" smallint NOT NULL DEFAULT 1002'
    )
    await cur.execute('ALTER TABLE "bench_organization" ALTER COLUMN "region" DROP DEFAULT')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "owned_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "owned_by_type" smallint')

    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "owned_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "owned_by_type" smallint')

    # bench_file
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "owned_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "owned_by_type" smallint')

    # bench_stream
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "owned_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "owned_by_type" smallint')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "owned_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "owned_by_type" smallint')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
