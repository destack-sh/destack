# This migration was automatically generated on 2025.03.06. Edit as needed.
import psycopg

ID = 37
VERSION = "2025.03.06.0"
HAS_GLOBAL = False
HAS_REGIONAL = True
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
    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "owned_by_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "owned_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "owned_by_bench_id"')

    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" DROP COLUMN "owned_by_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_browser" DROP COLUMN "owned_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_browser" DROP COLUMN "owned_by_bench_id"')

    # bench_file
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "owned_by_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "owned_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "owned_by_bench_id"')

    # bench_stream
    await cur.execute('ALTER TABLE "bench_stream" DROP COLUMN "owned_by_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_stream" DROP COLUMN "owned_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_stream" DROP COLUMN "owned_by_bench_id"')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "owned_by_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "owned_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "owned_by_bench_id"')


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_thread
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "owned_by_bench_id"')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "owned_by_bench_id"')

    # bench_kit
    await cur.execute('DROP INDEX "bench_implementation_bench_idx_parent_id"')

    # bench_package
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "owned_by_bench_id"')

    # bench_task
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "owned_by_bench_id"')

    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "owned_by_bench_id"')

    # bench_task
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "ck" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "template_at" timestamp')

    # bench_kit
    await cur.execute(
        'CREATE INDEX "bench_kit_bench_idx_parent_id" ON bench_kit USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE bench_task    
        ALTER COLUMN "type" SET DEFAULT 10,
        ALTER COLUMN "node_id" DROP NOT NULL,
        ALTER COLUMN "node_ck" DROP NOT NULL,
        ALTER COLUMN "node_type" DROP NOT NULL,
        ALTER COLUMN "node_bench_id" DROP NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
