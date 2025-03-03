# This migration was automatically generated on 2025.03.03. Edit as needed.
import psycopg

ID = 27
VERSION = "2025.03.03.1"
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
    # bench_choice
    await cur.execute(
        'ALTER TABLE "bench_choice" RENAME COLUMN "block_bench_id" TO "definition_bench_id"'
    )
    await cur.execute('ALTER TABLE "bench_choice" RENAME COLUMN "block_ck" TO "definition_ck"')
    await cur.execute('ALTER TABLE "bench_choice" RENAME COLUMN "block_id" TO "definition_id"')

    # bench_view
    await cur.execute(
        'ALTER TABLE "bench_view" RENAME COLUMN "block_bench_id" TO "definition_bench_id"'
    )
    await cur.execute('ALTER TABLE "bench_view" RENAME COLUMN "block_ck" TO "definition_ck"')
    await cur.execute('ALTER TABLE "bench_view" RENAME COLUMN "block_id" TO "definition_id"')

    # bench_channel
    await cur.execute(
        'ALTER TABLE "bench_channel" RENAME COLUMN "block_bench_id" TO "definition_bench_id"'
    )
    await cur.execute('ALTER TABLE "bench_channel" RENAME COLUMN "block_ck" TO "definition_ck"')
    await cur.execute('ALTER TABLE "bench_channel" RENAME COLUMN "block_id" TO "definition_id"')

    # bench_database
    await cur.execute(
        'ALTER TABLE "bench_database" RENAME COLUMN "block_bench_id" TO "definition_bench_id"'
    )
    await cur.execute('ALTER TABLE "bench_database" RENAME COLUMN "block_ck" TO "definition_ck"')
    await cur.execute('ALTER TABLE "bench_database" RENAME COLUMN "block_id" TO "definition_id"')

    # bench_role
    await cur.execute(
        'ALTER TABLE "bench_role" RENAME COLUMN "block_bench_id" TO "definition_bench_id"'
    )
    await cur.execute('ALTER TABLE "bench_role" RENAME COLUMN "block_ck" TO "definition_ck"')
    await cur.execute('ALTER TABLE "bench_role" RENAME COLUMN "block_id" TO "definition_id"')

    # bench_flow
    await cur.execute(
        'ALTER TABLE "bench_flow" RENAME COLUMN "block_bench_id" TO "definition_bench_id"'
    )
    await cur.execute('ALTER TABLE "bench_flow" RENAME COLUMN "block_ck" TO "definition_ck"')
    await cur.execute('ALTER TABLE "bench_flow" RENAME COLUMN "block_id" TO "definition_id"')

    # bench_class
    await cur.execute(
        'ALTER TABLE "bench_class" RENAME COLUMN "block_bench_id" TO "definition_bench_id"'
    )
    await cur.execute('ALTER TABLE "bench_class" RENAME COLUMN "block_ck" TO "definition_ck"')
    await cur.execute('ALTER TABLE "bench_class" RENAME COLUMN "block_id" TO "definition_id"')

    # bench_tag
    await cur.execute(
        'ALTER TABLE "bench_tag" RENAME COLUMN "block_bench_id" TO "definition_bench_id"'
    )
    await cur.execute('ALTER TABLE "bench_tag" RENAME COLUMN "block_ck" TO "definition_ck"')
    await cur.execute('ALTER TABLE "bench_tag" RENAME COLUMN "block_id" TO "definition_id"')

    # bench_implementation
    await cur.execute(
        'ALTER TABLE "bench_implementation" RENAME COLUMN "block_bench_id" TO "definition_bench_id"'
    )
    await cur.execute(
        'ALTER TABLE "bench_implementation" RENAME COLUMN "block_ck" TO "definition_ck"'
    )
    await cur.execute(
        'ALTER TABLE "bench_implementation" RENAME COLUMN "block_id" TO "definition_id"'
    )

    # bench_page
    await cur.execute(
        'ALTER TABLE "bench_page" RENAME COLUMN "block_bench_id" TO "definition_bench_id"'
    )
    await cur.execute('ALTER TABLE "bench_page" RENAME COLUMN "block_ck" TO "definition_ck"')
    await cur.execute('ALTER TABLE "bench_page" RENAME COLUMN "block_id" TO "definition_id"')
    # bench_task
    await cur.execute(
        'ALTER TABLE "bench_task" RENAME COLUMN "block_bench_id" TO "definition_bench_id"'
    )
    await cur.execute('ALTER TABLE "bench_task" RENAME COLUMN "block_ck" TO "definition_ck"')
    await cur.execute('ALTER TABLE "bench_task" RENAME COLUMN "block_id" TO "definition_id"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "package_id"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "tags_id"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "template_at"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "template_id"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "thread_id"')

    # bench_task - add new columns
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "session_id" uuid')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "run_id" uuid')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "run_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "client_id" uuid')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "machine_id" uuid')
    await cur.execute('ALTER TABLE "bench_task" ADD COLUMN "user_id" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
