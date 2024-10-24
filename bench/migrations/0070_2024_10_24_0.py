# This migration was automatically generated on 2024.10.24. Edit as needed.
import psycopg

ID = 70
VERSION = "2024.10.24.0"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_bench
    await cur.execute('ALTER TABLE "bench_bench" ADD COLUMN "subnode_packed" jsonb')

    # bench_user
    await cur.execute('ALTER TABLE "bench_user" ADD COLUMN "subnode_packed" jsonb')

    # bench_organization
    await cur.execute('ALTER TABLE "bench_organization" ADD COLUMN "subnode_packed" jsonb')

    # bench_handle
    await cur.execute('ALTER TABLE "bench_handle" ADD COLUMN "subnode_packed" jsonb')

    # bench_client
    await cur.execute('ALTER TABLE "bench_client" ADD COLUMN "subnode_packed" jsonb')

    # bench_server
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "subnode_packed" jsonb')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "subnode_packed" jsonb')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "subnode_packed" jsonb')

    # bench_drive
    await cur.execute('ALTER TABLE "bench_drive" ADD COLUMN "subnode_packed" jsonb')

    # bench_vault
    await cur.execute('ALTER TABLE "bench_vault" ADD COLUMN "subnode_packed" jsonb')

    # bench_cache
    await cur.execute('ALTER TABLE "bench_cache" ADD COLUMN "subnode_packed" jsonb')

    # bench_file
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "subnode_packed" jsonb')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "subnode_packed" jsonb')

    # bench_membership
    await cur.execute('ALTER TABLE "bench_membership" ADD COLUMN "subnode_packed" jsonb')

    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" ADD COLUMN "subnode_packed" jsonb')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "code"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "run_options"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "text"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "value_packed"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "value_type"')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "expansion"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "value_packed"')

    # bench_branch
    await cur.execute('ALTER TABLE "bench_branch" ADD COLUMN "subnode_packed" jsonb')

    # bench_package
    await cur.execute('ALTER TABLE "bench_package" ADD COLUMN "subnode_packed" jsonb')

    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" ADD COLUMN "subnode_packed" jsonb')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "subnode_packed" jsonb')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "subnode_packed" jsonb')

    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "subnode_packed" jsonb')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "subnode_packed" jsonb')

    # bench_query
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "subnode_packed" jsonb')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "subnode_packed" jsonb')
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "subviews_packed" jsonb')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "subnode_packed" jsonb')

    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "subnode_packed" jsonb')

    # bench_badge
    await cur.execute('ALTER TABLE "bench_badge" ADD COLUMN "subnode_packed" jsonb')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "subnode_packed" jsonb')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "type" smallint NOT NULL DEFAULT 1')

    # bench_session
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "subnode_packed" jsonb')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "subnode_packed" jsonb')

    # bench_signal
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "subnode_packed" jsonb')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "subnode_packed" jsonb')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "subnode_packed" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
