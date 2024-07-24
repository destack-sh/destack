# This migration was automatically generated on 2024.07.24. Edit as needed.
import psycopg

ID = 20
VERSION = "2024.07.24.5"
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
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "content"')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "inline_content" bytea')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
