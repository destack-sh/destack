# This migration was automatically generated on 2024.07.09. Edit as needed.
import psycopg

ID = 5
VERSION = "2024.07.09.2"
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
    # bench_run
    await cur.execute(
        'ALTER TABLE "bench_run" ADD COLUMN "logs" jsonb[] NOT NULL DEFAULT ARRAY[]::jsonb[];'
    )
    await cur.execute('ALTER TABLE "bench_run" ALTER COLUMN "logs" DROP DEFAULT')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
