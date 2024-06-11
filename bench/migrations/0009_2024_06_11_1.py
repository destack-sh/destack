# This migration was automatically generated on 2024.06.11. Edit as needed.
import psycopg

ID = 9
VERSION = "2024.06.11.1"
HAS_GLOBAL = False
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "block_bench_id"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "block_ck"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "block_id"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "client_id"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "machine_id"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "run_id"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "run_root_base_ck"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "run_root_id"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "server_id"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "session_id"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "step_bench_id"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "step_ck"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "step_id"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "user_id"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "block_bench_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "block_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "block_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "client_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "machine_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "run_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "run_root_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "run_root_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "server_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "session_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "step_bench_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "step_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "step_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "user_id"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
