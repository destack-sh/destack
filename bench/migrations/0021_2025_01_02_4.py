# This migration was automatically generated on 2025.01.02. Edit as needed.
import psycopg

ID = 21
VERSION = "2025.01.02.4"
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
    # bench_query
    await cur.execute('DROP TABLE "bench_query"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
