# This migration was automatically generated on 2024.11.15. Edit as needed.
import psycopg

ID = 81
VERSION = "2024.11.15.2"
HAS_GLOBAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "signal_bench_id"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "signal_ck"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "signal_id"')
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "parent_type" smallint')
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "parent_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "message_id" uuid')
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "message_ck" uuid')
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "message_bench_id" uuid')

    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "run_options" jsonb')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "status" smallint NOT NULL DEFAULT 4')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "interrupted_at" timestamp')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "interrupt_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "interrupt_ck" uuid')

    # bench_interrupt
    await cur.execute(
        """
    CREATE TABLE "bench_interrupt" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_base_ck" uuid,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "kind" smallint NOT NULL,
        "root_id" uuid,
        "root_base_ck" uuid,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "step_id" uuid,
        "step_ck" uuid,
        "step_bench_id" uuid,
        "pipe_id" uuid,
        "pipe_ck" uuid,
        "pipe_bench_id" uuid,
        "status" smallint NOT NULL DEFAULT 1,
        "duration" interval,
        "opened_at" timestamp,
        "closed_at" timestamp,
        "trigger_id" uuid,
        "trigger_ck" uuid,
        "trigger_bench_id" uuid,
        "session_id" uuid,
        "run_id" uuid,
        "run_base_ck" uuid,
        "run_root_id" uuid,
        "run_root_base_ck" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "server_id" uuid,
        "user_id" uuid,
        "identity_id" uuid,
        "identity_ck" uuid,
        "identity_bench_id" uuid
    )
    """
    )
    await cur.execute(
        'CREATE INDEX "bench_interrupt_bench_idx_created_at" ON bench_interrupt USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_interrupt_bench_idx_created_epoch" ON bench_interrupt USING BTREE (created_epoch)'
    )
    await cur.execute(
        'CREATE INDEX "bench_interrupt_bench_idx_package_id_created_at" ON bench_interrupt USING BTREE (package_id, created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_interrupt_bench_idx_package_id_created_epoch" ON bench_interrupt USING BTREE (package_id, created_epoch)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
