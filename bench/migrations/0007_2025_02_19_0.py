# This migration was automatically generated on 2025.02.19. Edit as needed.
import psycopg

ID = 7
VERSION = "2025.02.19.0"
HAS_GLOBAL = False
HAS_REGIONAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    pass


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_thread
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "run_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "run_bench_id"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "interruption_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "interruption_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "interruption_bench_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "interruption_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "spawned_thread_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "spawned_thread_id"')

    # bench_choice
    await cur.execute(
        'ALTER TABLE "bench_choice" ADD COLUMN "order_key" varchar NOT NULL DEFAULT \'a0\'::character varying'
    )

    # bench_class
    await cur.execute(
        'ALTER TABLE "bench_class" ADD COLUMN "order_key" varchar NOT NULL DEFAULT \'a0\'::character varying'
    )

    # bench_flow
    await cur.execute(
        'ALTER TABLE "bench_flow" ADD COLUMN "order_key" varchar NOT NULL DEFAULT \'a0\'::character varying'
    )

    # bench_database
    await cur.execute(
        'ALTER TABLE "bench_database" ADD COLUMN "order_key" varchar NOT NULL DEFAULT \'a0\'::character varying'
    )

    # bench_role
    await cur.execute(
        'ALTER TABLE "bench_role" ADD COLUMN "order_key" varchar NOT NULL DEFAULT \'a0\'::character varying'
    )

    # bench_identity
    await cur.execute(
        'ALTER TABLE "bench_identity" ADD COLUMN "order_key" varchar NOT NULL DEFAULT \'a0\'::character varying'
    )

    # bench_thread
    await cur.execute('ALTER TABLE "bench_thread" ADD COLUMN "run_root_id" uuid')
    await cur.execute('ALTER TABLE "bench_thread" ADD COLUMN "run_root_base_ck" uuid')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_root_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_root_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "selection" jsonb')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "created_interruption_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "created_interruption_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "created_interruption_base_ck" uuid')
    await cur.execute(
        'ALTER TABLE "bench_message" ADD COLUMN "created_interruption_base_bench_id" uuid'
    )
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "created_run_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "created_run_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "created_run_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "created_run_base_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "created_thread_id" uuid')

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE bench_thread    
        ALTER COLUMN "channel_id" SET NOT NULL,
        ALTER COLUMN "channel_ck" SET NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
