# This migration was automatically generated on 2025.01.14. Edit as needed.
import psycopg

ID = 28
VERSION = "2025.01.14.0"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_client
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "updated_epoch"')

    # bench_membership
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "updated_epoch"')

    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "updated_epoch"')

    # bench_bench
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "updated_epoch"')

    # bench_handle
    await cur.execute('ALTER TABLE "bench_handle" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_handle" DROP COLUMN "updated_epoch"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_store
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "updated_epoch"')
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "created_epoch"')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "updated_epoch"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "created_epoch"')

    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" DROP COLUMN "updated_epoch"')
    await cur.execute('ALTER TABLE "bench_browser" DROP COLUMN "created_epoch"')

    # bench_file
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "updated_epoch"')
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "created_epoch"')

    # bench_stream
    await cur.execute('ALTER TABLE "bench_stream" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_stream" DROP COLUMN "updated_epoch"')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "updated_epoch"')

    # bench_scaler
    await cur.execute('ALTER TABLE "bench_scaler" DROP COLUMN "updated_epoch"')
    await cur.execute('ALTER TABLE "bench_scaler" DROP COLUMN "created_epoch"')


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_package
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "updated_epoch"')

    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "updated_epoch"')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "updated_epoch"')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "updated_epoch"')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "updated_epoch"')

    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "updated_epoch"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "updated_epoch"')

    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "updated_epoch"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "scheduled_epoch"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "started_epoch"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "terminated_epoch"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "updated_epoch"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "updated_epoch"')

    # bench_action
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "updated_epoch"')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "updated_epoch"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "created_epoch"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "template_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "template_base_ck"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "updated_epoch"')

    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" ADD COLUMN "template_at" timestamp')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "template_at" timestamp')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "template_at" timestamp')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "template_at" timestamp')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "template_at" timestamp')

    # bench_action
    await cur.execute('ALTER TABLE "bench_action" ADD COLUMN "template_at" timestamp')

    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "template_at" timestamp')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
