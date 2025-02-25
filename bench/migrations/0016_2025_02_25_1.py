# This migration was automatically generated on 2025.02.25. Edit as needed.
import psycopg

ID = 16
VERSION = "2025.02.25.1"
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
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "default_channel_ck"')
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "default_channel_id"')
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "main_channel_id" uuid')
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "main_channel_ck" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
