# This migration was automatically generated on 2024.07.11. Edit as needed.
import psycopg

ID = 6
VERSION = "2024.07.11.1"
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
    # bench_issue
    await cur.execute('DROP TABLE "bench_issue"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
