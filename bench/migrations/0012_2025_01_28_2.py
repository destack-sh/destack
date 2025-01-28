# This migration was automatically generated on 2025.01.28. Edit as needed.
import psycopg

ID = 12
VERSION = "2025.01.28.2"
HAS_GLOBAL = True
HAS_REGIONAL = False
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_client
    await cur.execute('ALTER TABLE "bench_client" ADD COLUMN "bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_client" ADD COLUMN "user_id" uuid')


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
