# This migration was automatically generated on 2025.01.04. Edit as needed.
import psycopg

ID = 25
VERSION = "2025.01.04.0"
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
    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "mapping"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "context"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
