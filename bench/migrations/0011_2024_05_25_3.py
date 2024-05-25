# This migration was automatically generated on 2024.05.25. Edit as needed.
import psycopg

ID = 11
VERSION = "2024.05.25.3"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_badge
    await cur.execute('ALTER TABLE "bench_badge" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_badge" DROP COLUMN "updated_by_ck"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "updated_by_ck"')

    # bench_bench
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "updated_by_ck"')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "updated_by_ck"')

    # bench_branch
    await cur.execute('ALTER TABLE "bench_branch" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_branch" DROP COLUMN "updated_by_ck"')

    # bench_client
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "updated_by_ck"')

    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "updated_by_ck"')

    # bench_drive
    await cur.execute('ALTER TABLE "bench_drive" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_drive" DROP COLUMN "updated_by_ck"')

    # bench_environment
    await cur.execute('ALTER TABLE "bench_environment" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_environment" DROP COLUMN "updated_by_ck"')

    # bench_identity
    await cur.execute('ALTER TABLE "bench_identity" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_identity" DROP COLUMN "updated_by_ck"')

    # bench_handle
    await cur.execute('ALTER TABLE "bench_handle" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_handle" DROP COLUMN "updated_by_ck"')

    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "updated_by_ck"')

    # bench_link
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "updated_by_ck"')

    # bench_membership
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "updated_by_ck"')

    # bench_notice
    await cur.execute('ALTER TABLE "bench_notice" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_notice" DROP COLUMN "updated_by_ck"')

    # bench_organization
    await cur.execute('ALTER TABLE "bench_organization" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_organization" DROP COLUMN "updated_by_ck"')

    # bench_package
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "updated_by_ck"')

    # bench_query
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "updated_by_ck"')

    # bench_role
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "updated_by_ck"')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "updated_by_ck"')

    # bench_upgrade
    await cur.execute('ALTER TABLE "bench_upgrade" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_upgrade" DROP COLUMN "updated_by_ck"')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "updated_by_ck"')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "updated_by_ck"')

    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "updated_by_ck"')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "updated_by_ck"')

    # bench_user
    await cur.execute('ALTER TABLE "bench_user" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_user" DROP COLUMN "updated_by_ck"')

    # bench_server
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "updated_by_ck"')

    # bench_blob
    await cur.execute('ALTER TABLE "bench_blob" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_blob" DROP COLUMN "updated_by_ck"')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "updated_by_ck"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "created_by_ck" uuid')

    # bench_blob
    await cur.execute('ALTER TABLE "bench_blob" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_blob" ADD COLUMN "created_by_ck" uuid')

    # bench_server
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "created_by_ck" uuid')

    # bench_user
    await cur.execute('ALTER TABLE "bench_user" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_user" ADD COLUMN "created_by_ck" uuid')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "created_by_ck" uuid')

    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "created_by_ck" uuid')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "created_by_ck" uuid')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "created_by_ck" uuid')

    # bench_upgrade
    await cur.execute('ALTER TABLE "bench_upgrade" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_upgrade" ADD COLUMN "created_by_ck" uuid')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "created_by_ck" uuid')

    # bench_role
    await cur.execute('ALTER TABLE "bench_role" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_role" ADD COLUMN "created_by_ck" uuid')

    # bench_query
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "created_by_ck" uuid')

    # bench_package
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "created_by_ck" uuid')

    # bench_organization
    await cur.execute('ALTER TABLE "bench_organization" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_organization" ADD COLUMN "created_by_ck" uuid')

    # bench_notice
    await cur.execute('ALTER TABLE "bench_notice" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_notice" ADD COLUMN "created_by_ck" uuid')

    # bench_membership
    await cur.execute('ALTER TABLE "bench_membership" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_membership" ADD COLUMN "created_by_ck" uuid')

    # bench_link
    await cur.execute('ALTER TABLE "bench_link" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_link" ADD COLUMN "created_by_ck" uuid')

    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_invite" ADD COLUMN "created_by_ck" uuid')

    # bench_handle
    await cur.execute('ALTER TABLE "bench_handle" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_handle" ADD COLUMN "created_by_ck" uuid')

    # bench_identity
    await cur.execute('ALTER TABLE "bench_identity" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_identity" ADD COLUMN "created_by_ck" uuid')

    # bench_environment
    await cur.execute('ALTER TABLE "bench_environment" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_environment" ADD COLUMN "created_by_ck" uuid')

    # bench_drive
    await cur.execute('ALTER TABLE "bench_drive" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_drive" ADD COLUMN "created_by_ck" uuid')

    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_dependency" ADD COLUMN "created_by_ck" uuid')

    # bench_client
    await cur.execute('ALTER TABLE "bench_client" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_client" ADD COLUMN "created_by_ck" uuid')

    # bench_branch
    await cur.execute('ALTER TABLE "bench_branch" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_branch" ADD COLUMN "created_by_ck" uuid')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "created_by_ck" uuid')

    # bench_bench
    await cur.execute('ALTER TABLE "bench_bench" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_bench" ADD COLUMN "created_by_ck" uuid')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "created_by_ck" uuid')

    # bench_badge
    await cur.execute('ALTER TABLE "bench_badge" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_badge" ADD COLUMN "created_by_ck" uuid')


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "updated_by_ck"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "root_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "run_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "session_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "updated_by_ck"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "run_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "session_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "updated_by_ck"')

    # bench_signal
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "run_ck"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "session_ck"')
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "updated_by_ck"')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "run_ck"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "session_ck"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "updated_by_ck"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "reply_to_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "run_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "session_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "updated_by_ck"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "session_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "reply_to_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "created_by_ck" uuid')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "session_ck" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "run_ck" uuid')
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "created_by_ck" uuid')

    # bench_signal
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "session_ck" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "run_ck" uuid')
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "created_by_ck" uuid')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "session_ck" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "run_ck" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "created_by_ck" uuid')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "session_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "run_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "root_ck" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "created_by_ck" uuid')

    # bench_session
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "updated_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "created_by_ck" uuid')
