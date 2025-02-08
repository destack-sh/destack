# This migration was automatically generated on 2025.02.07. Edit as needed.
import psycopg

ID = 9
VERSION = "2025.02.07.3"
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
    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "line"')
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "text" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
