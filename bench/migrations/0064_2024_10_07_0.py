# This migration was automatically generated on 2024.10.07. Edit as needed.
import psycopg

ID = 64
VERSION = "2024.10.07.0"
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
    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "properties"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
