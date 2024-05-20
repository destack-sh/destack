# This migration was automatically generated on 2024.05.20. Edit as needed.
import psycopg

ID = 6
VERSION = "2024.05.20.3"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_client
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "main_space_bench_id"')
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "main_space_ck"')
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "main_space_id"')

    # bench_server
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "version"')

    # bench_machine
    await cur.execute(
        """
    CREATE TABLE "bench_machine" (
        "id" uuid NOT NULL PRIMARY KEY,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL DEFAULT 0,
        "created_at" timestamp NOT NULL,
        "updated_at" timestamp NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "name" varchar NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL DEFAULT 1,
        "status" smallint NOT NULL DEFAULT 1,
        "profile" smallint NOT NULL,
        "version" varchar,
        "external_name" varchar,
        "external_id" varchar,
        "connection_uri" bytea,
        "started_at" timestamp,
        "terminated_at" timestamp,
        "active_at" timestamp,
        "bumped_at" timestamp
    )
    """
    )

    # bench_client
    await cur.execute('ALTER TABLE "bench_client" ADD COLUMN "space_id" uuid')
    await cur.execute('ALTER TABLE "bench_client" ADD COLUMN "space_ck" uuid')
    await cur.execute('ALTER TABLE "bench_client" ADD COLUMN "space_bench_id" uuid')

    # bench_machine
    await cur.execute(
        'ALTER TABLE "bench_machine" ADD COLUMN "parent_server_id" uuid REFERENCES bench_server ON DELETE CASCADE'
    )
    await cur.execute(
        'ALTER TABLE "bench_machine" ADD CONSTRAINT "bench_machine_bench_check_one_parent" CHECK ((parent_server_id IS NOT NULL))'
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_session
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "machine_id" uuid')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "machine_bench_id" uuid')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "machine_id" uuid')

    # bench_signal
    await cur.execute('ALTER TABLE "bench_signal" ADD COLUMN "machine_id" uuid')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "machine_id" uuid')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "machine_id" uuid')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "machine_id" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
