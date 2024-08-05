# This migration was automatically generated on 2024.08.05. Edit as needed.
import psycopg

ID = 29
VERSION = "2024.08.05.3"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_server
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "cpu"')
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "current_cpu"')
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "current_ram"')
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "ram"')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "min_cpu" real')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "max_cpu" real')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "min_ram" real')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "max_ram" real')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
