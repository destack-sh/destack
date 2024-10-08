# This migration was automatically generated on 2024.10.08. Edit as needed.
import psycopg

ID = 66
VERSION = "2024.10.08.2"
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
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "new_revision"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
