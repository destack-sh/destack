# This migration was automatically generated on 2024.11.07. Edit as needed.
import psycopg

ID = 77
VERSION = "2024.11.07.1"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" DROP COLUMN "name"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
