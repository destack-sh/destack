# This migration was automatically generated on 2024.07.11. Edit as needed.
import psycopg

ID = 9
VERSION = "2024.07.11.6"
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
    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "is_page"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
