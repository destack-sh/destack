# This migration was automatically generated on 2024.05.19. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.05.19.0"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_bench
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "set_properties"')

    # bench_branch
    await cur.execute('ALTER TABLE "bench_branch" DROP COLUMN "set_properties"')

    # bench_client
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "set_properties"')

    # bench_drive
    await cur.execute('ALTER TABLE "bench_drive" DROP COLUMN "set_properties"')

    # bench_environment
    await cur.execute('ALTER TABLE "bench_environment" DROP COLUMN "set_properties"')

    # bench_handle
    await cur.execute('ALTER TABLE "bench_handle" DROP COLUMN "set_properties"')

    # bench_organization
    await cur.execute('ALTER TABLE "bench_organization" DROP COLUMN "set_properties"')

    # bench_package
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "set_properties"')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "set_properties"')

    # bench_user
    await cur.execute('ALTER TABLE "bench_user" DROP COLUMN "set_properties"')

    # bench_server
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "set_properties"')

    # bench_blob
    await cur.execute('ALTER TABLE "bench_blob" DROP COLUMN "set_properties"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "request"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "set_properties"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "set_properties"')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "set_properties"')

    # bench_record_shared
    await cur.execute('DROP INDEX "bench_record_shared_bench_idx_block_ck_deleted_at"')
    await cur.execute('DROP INDEX "bench_record_shared_bench_idx_block_key_archived_at"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "set_properties"')

    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "set_properties"')

    # bench_signal
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "set_properties"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "type" smallint')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "properties" integer[] NOT NULL')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "new_node_packed" jsonb')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "old_node_packed" jsonb')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "secret_value_packed" bytea')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "node_id" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "node_ck" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "node_type" smallint')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "node_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "node_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "node_base_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "client_id" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "server_id" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "user_id" uuid')
    await cur.execute(
        """
        ALTER TABLE bench_log    
        ALTER COLUMN level SET DEFAULT 3
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
