# This migration was automatically generated on 2024.12.11. Edit as needed.
import psycopg

ID = 18
VERSION = "2024.12.11.1"
HAS_GLOBAL = True
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
    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "is_name_hidden"')
    await cur.execute(
        'ALTER TABLE "bench_pipe" ADD COLUMN "is_name_shown" boolean NOT NULL DEFAULT false'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
