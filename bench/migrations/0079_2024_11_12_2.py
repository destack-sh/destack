# This migration was automatically generated on 2024.11.12. Edit as needed.
import psycopg

ID = 79
VERSION = "2024.11.12.2"
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
    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "path"')

    # bench_notification
    await cur.execute('DROP TABLE "bench_notification"')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "bar_position"')

    # bench_signal
    await cur.execute('DROP TABLE "bench_signal"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "block_id" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "block_ck" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "block_bench_id" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "reply_to_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "reply_to_base_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "expires_at" timestamp')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "read_at" timestamp')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
