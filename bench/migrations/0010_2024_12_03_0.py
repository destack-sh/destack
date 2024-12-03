# This migration was automatically generated on 2024.12.03. Edit as needed.
import psycopg

ID = 10
VERSION = "2024.12.03.0"
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
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "is_builtin"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "is_owned"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
