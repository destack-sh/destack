# This migration was automatically generated on 2025.01.20. Edit as needed.
import psycopg

ID = 2
VERSION = "2025.01.20.0"
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
    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "run_root_base_ck"')
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "run_root_id"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "attempts"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "events"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "logs"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "run_root_base_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "run_root_id"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "spans"')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "run_root_base_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "run_root_id"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "category"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "change_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "kind"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "node_base_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "node_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "node_data"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "node_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "node_type"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "operations"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "run_root_base_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "run_root_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "undo_of_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "vignette"')

    # bench_run_span
    await cur.execute(
        """
    CREATE TABLE "bench_run_span" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_base_ck" uuid,
        "bench_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "root_id" uuid,
        "root_base_ck" uuid,
        "level" smallint NOT NULL DEFAULT 3,
        "status" smallint NOT NULL DEFAULT 1,
        "duration" interval,
        "started_at" timestamp,
        "terminated_at" timestamp,
        "interrupted_at" timestamp,
        "interruption_id" uuid,
        "interruption_base_ck" uuid,
        "error" jsonb,
        "title" varchar,
        "text" jsonb,
        "nodes_id" uuid[],
        "nodes_ck" uuid[],
        "nodes_type" smallint[],
        "nodes_bench_id" uuid[],
        "nodes_base_ck" uuid[],
        "nodes_base_bench_id" uuid[],
        "session_id" uuid,
        "run_id" uuid,
        "run_base_ck" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "user_id" uuid,
        "identity_id" uuid,
        "identity_ck" uuid,
        "identity_bench_id" uuid
    )
    """
    )

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "parent_type" smallint')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "parent_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "title" varchar')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "text" jsonb')

    # bench_run_span
    await cur.execute(
        'CREATE INDEX "bench_run_span_bench_idx_created_at" ON bench_run_span USING BTREE (created_at)'
    )

    # bench_log
    await cur.execute(
        """
        ALTER TABLE bench_log    
        ALTER COLUMN "type" SET NOT NULL,
        ALTER COLUMN "level" DROP DEFAULT
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
