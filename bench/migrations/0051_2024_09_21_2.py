# This migration was automatically generated on 2024.09.21. Edit as needed.
import psycopg

ID = 51
VERSION = "2024.09.21.2"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_file
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "name"')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "name"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
