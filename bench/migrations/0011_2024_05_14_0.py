# This migration was automatically generated on 2024.05.14. Edit as needed.
import psycopg

ID = 11
VERSION = "2024.05.14.0"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_store
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "database"')
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "host"')
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "main_credential"')
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "schema"')
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "external_name" varchar')
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "external_id" varchar')
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "connection_uri" bytea')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_migration
    await cur.execute(
        """
    CREATE TABLE "bench_migration" (
        "id" integer NOT NULL PRIMARY KEY,
        "version" varchar NOT NULL,
        "has_global" boolean NOT NULL,
        "has_local" boolean NOT NULL,
        "applied_at" timestamp
    )
    """
    )

    # bench_record_ephemeral
    await cur.execute(
        """
    CREATE TABLE "bench_record_ephemeral" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "revision" bigint NOT NULL DEFAULT 0,
        "created_at" timestamp NOT NULL DEFAULT now(),
        "updated_at" timestamp NOT NULL DEFAULT now(),
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "block_tk" uuid NOT NULL,
        "block_ck" uuid NOT NULL,
        "block_id" uuid NOT NULL,
        "value_packed" jsonb,
        "secret_value_packed" bytea
    )
    """
    )

    # bench_session
    await cur.execute(
        """
    CREATE TABLE "bench_session" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_package_id" uuid,
        "package_id" uuid NOT NULL,
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
        "set_properties" integer[] NOT NULL,
        "status" smallint NOT NULL DEFAULT 1,
        "duration" real,
        "opened_at" timestamp,
        "closed_at" timestamp,
        "client_id" uuid,
        "client_bench_id" uuid,
        "server_id" uuid,
        "server_bench_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_run
    await cur.execute(
        """
    CREATE TABLE "bench_run" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_package_id" uuid,
        "package_id" uuid NOT NULL,
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
        "set_properties" integer[] NOT NULL,
        "kind" smallint NOT NULL,
        "root_id" uuid,
        "root_ck" uuid,
        "root_bench_id" uuid,
        "root_base_ck" uuid,
        "root_base_bench_id" uuid,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "step_id" uuid,
        "step_ck" uuid,
        "step_bench_id" uuid,
        "code" jsonb,
        "text" jsonb,
        "status" smallint NOT NULL DEFAULT 1,
        "duration" real,
        "scheduled_at" timestamp,
        "started_at" timestamp,
        "paused_at" timestamp,
        "terminated_at" timestamp,
        "inputs_packed" jsonb,
        "inputs_secret_packed" bytea,
        "outputs_packed" jsonb,
        "outputs_secret_packed" bytea,
        "value_packed" jsonb,
        "value_secret_packed" bytea,
        "error" jsonb,
        "session_id" uuid,
        "session_ck" uuid,
        "session_bench_id" uuid,
        "client_id" uuid,
        "client_bench_id" uuid,
        "server_id" uuid,
        "server_bench_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_signal
    await cur.execute(
        """
    CREATE TABLE "bench_signal" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_package_id" uuid,
        "package_id" uuid NOT NULL,
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
        "set_properties" integer[] NOT NULL,
        "type_id" uuid,
        "type_ck" uuid,
        "type_bench_id" uuid,
        "origin_id" uuid,
        "origin_ck" uuid,
        "origin_bench_id" uuid,
        "value_packed" jsonb,
        "secret_value_packed" bytea
    )
    """
    )

    # bench_log
    await cur.execute(
        """
    CREATE TABLE "bench_log" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_package_id" uuid,
        "package_id" uuid NOT NULL,
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
        "set_properties" integer[] NOT NULL,
        "kind" smallint NOT NULL,
        "level" smallint NOT NULL,
        "logger" varchar,
        "event" varchar,
        "title" varchar,
        "text" jsonb,
        "value_packed" jsonb,
        "request" jsonb,
        "session_id" uuid,
        "session_ck" uuid,
        "session_bench_id" uuid,
        "run_id" uuid,
        "run_ck" uuid,
        "run_bench_id" uuid,
        "run_base_ck" uuid,
        "run_base_bench_id" uuid,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "step_id" uuid,
        "step_ck" uuid,
        "step_bench_id" uuid
    )
    """
    )

    # bench_notification
    await cur.execute(
        """
    CREATE TABLE "bench_notification" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_package_id" uuid,
        "package_id" uuid NOT NULL,
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
        "set_properties" integer[] NOT NULL,
        "kind" smallint NOT NULL,
        "type_id" uuid,
        "type_ck" uuid,
        "type_bench_id" uuid,
        "expires_at" timestamp,
        "read_at" timestamp,
        "origin_id" uuid,
        "origin_ck" uuid,
        "origin_bench_id" uuid,
        "title" varchar,
        "text" jsonb,
        "value_packed" jsonb,
        "secret_value_packed" bytea
    )
    """
    )

    # bench_record_ephemeral
    await cur.execute(
        'CREATE INDEX "bench_record_ephemeral_bench_idx_block_ck_deleted_at" ON bench_record_ephemeral USING BTREE (block_ck, deleted_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_record_ephemeral_bench_idx_block_key_archived_at" ON bench_record_ephemeral USING BTREE (archived_at, block_ck)'
    )

    # bench_session
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_status" ON bench_session USING BTREE (status)'
    )
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_package_deleted_at" ON bench_session USING BTREE (deleted_at, package_id)'
    )
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_package_archived_at" ON bench_session USING BTREE (archived_at, package_id)'
    )
    await cur.execute(
        'ALTER TABLE "bench_session" ADD CONSTRAINT "bench_session_bench_check_one_parent" CHECK ((parent_package_id IS NOT NULL))'
    )

    # bench_run
    await cur.execute(
        'ALTER TABLE "bench_run" ADD COLUMN "parent_run_id" uuid REFERENCES bench_run ON DELETE CASCADE'
    )
    await cur.execute('CREATE INDEX "bench_run_bench_idx_status" ON bench_run USING BTREE (status)')
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_package_deleted_at" ON bench_run USING BTREE (deleted_at, package_id)'
    )
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_package_archived_at" ON bench_run USING BTREE (archived_at, package_id)'
    )
    await cur.execute(
        'ALTER TABLE "bench_run" ADD CONSTRAINT "bench_run_bench_check_one_parent" CHECK ((parent_package_id IS NOT NULL) OR (parent_run_id IS NOT NULL))'
    )

    # bench_signal
    await cur.execute(
        'CREATE INDEX "bench_signal_bench_idx_package_deleted_at" ON bench_signal USING BTREE (deleted_at, package_id)'
    )
    await cur.execute(
        'CREATE INDEX "bench_signal_bench_idx_package_archived_at" ON bench_signal USING BTREE (archived_at, package_id)'
    )
    await cur.execute(
        'ALTER TABLE "bench_signal" ADD CONSTRAINT "bench_signal_bench_check_one_parent" CHECK ((parent_package_id IS NOT NULL))'
    )

    # bench_log
    await cur.execute(
        'CREATE INDEX "bench_log_bench_idx_package_deleted_at" ON bench_log USING BTREE (deleted_at, package_id)'
    )
    await cur.execute(
        'CREATE INDEX "bench_log_bench_idx_package_archived_at" ON bench_log USING BTREE (archived_at, package_id)'
    )
    await cur.execute(
        'ALTER TABLE "bench_log" ADD CONSTRAINT "bench_log_bench_check_one_parent" CHECK ((parent_package_id IS NOT NULL))'
    )

    # bench_notification
    await cur.execute(
        'CREATE INDEX "bench_notification_bench_idx_package_deleted_at" ON bench_notification USING BTREE (deleted_at, package_id)'
    )
    await cur.execute(
        'CREATE INDEX "bench_notification_bench_idx_package_archived_at" ON bench_notification USING BTREE (archived_at, package_id)'
    )
    await cur.execute(
        'ALTER TABLE "bench_notification" ADD CONSTRAINT "bench_notification_bench_check_one_parent" CHECK ((parent_package_id IS NOT NULL))'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
