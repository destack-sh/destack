# This migration was automatically generated on 2024.07.24. Edit as needed.
import psycopg

ID = 18
VERSION = "2024.07.24.3"
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
    # bench_link
    await cur.execute('DROP TABLE "bench_link"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
