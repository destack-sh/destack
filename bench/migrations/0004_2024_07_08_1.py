# This migration was automatically generated on 2024.07.08. Edit as needed.
import psycopg

ID = 4
VERSION = "2024.07.08.1"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_blob
    await cur.execute('ALTER TABLE "bench_blob" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_blob" DROP COLUMN "updated_by_base_ck"')

    # bench_bench
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "updated_by_base_ck"')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "updated_by_base_ck"')

    # bench_server
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "updated_by_base_ck"')

    # bench_drive
    await cur.execute('ALTER TABLE "bench_drive" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_drive" DROP COLUMN "updated_by_base_ck"')

    # bench_user
    await cur.execute('ALTER TABLE "bench_user" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_user" DROP COLUMN "updated_by_base_ck"')

    # bench_organization
    await cur.execute('ALTER TABLE "bench_organization" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_organization" DROP COLUMN "updated_by_base_ck"')

    # bench_handle
    await cur.execute('ALTER TABLE "bench_handle" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_handle" DROP COLUMN "updated_by_base_ck"')

    # bench_client
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "updated_by_base_ck"')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "updated_by_base_ck"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "updated_by_base_ck"')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "updated_by_base_ck"')

    # bench_link
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "updated_by_base_ck"')

    # bench_issue
    await cur.execute('ALTER TABLE "bench_issue" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_issue" DROP COLUMN "updated_by_base_ck"')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "updated_by_base_ck"')

    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "updated_by_base_ck"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "updated_by_base_ck"')

    # bench_query
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "updated_by_base_ck"')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "updated_by_base_ck"')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "updated_by_base_ck"')

    # bench_badge
    await cur.execute('ALTER TABLE "bench_badge" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_badge" DROP COLUMN "updated_by_base_ck"')

    # bench_membership
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "updated_by_base_ck"')

    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "updated_by_base_ck"')

    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "updated_by_base_ck"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "updated_by_base_ck"')

    # bench_signal
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "updated_by_base_ck"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "updated_by_base_ck"')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "updated_by_base_ck"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "updated_by_base_ck"')

    # bench_branch
    await cur.execute('ALTER TABLE "bench_branch" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_branch" DROP COLUMN "updated_by_base_ck"')

    # bench_package
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "created_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "updated_by_base_ck"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
