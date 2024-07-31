# This migration was automatically generated on 2024.07.31. Edit as needed.
import psycopg

ID = 24
VERSION = "2024.07.31.1"
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
    # bench_file
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "url"')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "external_url" varchar')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
