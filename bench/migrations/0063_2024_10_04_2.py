# This migration was automatically generated on 2024.10.04. Edit as needed.
import psycopg

ID = 63
VERSION = "2024.10.04.2"
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
    # bench_log
    await cur.execute('TRUNCATE TABLE "bench_log"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "new_node_packed"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "new_node_secret_packed"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "old_node_packed"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "old_node_secret_packed"')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "node_data" jsonb')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "operations" jsonb[] NOT NULL')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
