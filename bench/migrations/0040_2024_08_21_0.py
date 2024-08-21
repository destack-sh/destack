# This migration was automatically generated on 2024.08.21. Edit as needed.
import psycopg

ID = 40
VERSION = "2024.08.21.0"
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
    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "ports"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
