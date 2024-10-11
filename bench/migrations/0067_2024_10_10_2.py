# This migration was automatically generated on 2024.10.11. Edit as needed.
import psycopg

ID = 67
VERSION = "2024.10.10.2"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_file
    await cur.execute('ALTER TABLE "bench_file" RENAME COLUMN "coarse_type" TO "type"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "pipe_bench_id"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "pipe_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "pipe_id"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "view_bench_id"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "view_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "view_id"')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "kind"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "type_bench_id"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "type_ck"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "type_id"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "block_bench_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "block_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "block_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "pipe_bench_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "pipe_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "pipe_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "step_bench_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "step_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "step_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "view_bench_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "view_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "view_id"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" RENAME COLUMN "zone" TO "type"')
    await cur.execute(
        'ALTER TABLE "bench_field" RENAME COLUMN "base_field_zone" TO "base_field_type"'
    )

    # bench_signal
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "pipe_bench_id"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "pipe_ck"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "pipe_id"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "step_bench_id"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "step_ck"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "step_id"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "type_bench_id"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "type_ck"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "type_id"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "view_bench_id"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "view_ck"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "view_id"')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "block_id" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "block_ck" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "block_bench_id" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "level" smallint NOT NULL')

    # bench_signal
    await cur.execute(
        """
        ALTER TABLE bench_signal    
        ALTER COLUMN block_id SET NOT NULL,
        ALTER COLUMN block_ck SET NOT NULL,
        ALTER COLUMN block_bench_id SET NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
