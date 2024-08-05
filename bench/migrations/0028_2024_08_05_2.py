# This migration was automatically generated on 2024.08.05. Edit as needed.
import psycopg

ID = 28
VERSION = "2024.08.05.2"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_server
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "current_profile"')
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "profile"')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "current_profile"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "profile"')

    # bench_server
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "cpu" real')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "current_cpu" real')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "ram" real')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "current_ram" real')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "cpu" real NOT NULL')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "current_cpu" real')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "ram" integer NOT NULL')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "current_ram" integer')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
