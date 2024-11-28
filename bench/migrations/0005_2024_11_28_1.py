# This migration was automatically generated on 2024.11.28. Edit as needed.
import psycopg

ID = 5
VERSION = "2024.11.28.1"
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
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" RENAME COLUMN "kind" TO "type"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
