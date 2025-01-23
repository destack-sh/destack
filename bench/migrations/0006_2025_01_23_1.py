# This migration was automatically generated on 2025.01.23. Edit as needed.
import psycopg

ID = 6
VERSION = "2025.01.23.1"
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
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "caller_base_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "caller_id"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "outgoing_base_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "outgoing_id"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "level"')

    # bench_run_span
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "level"')

    # bench_action
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "call"')
    await cur.execute('ALTER TABLE "bench_action" ADD COLUMN "plans" jsonb[]')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "plan_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "plan_step" integer')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "attempt" integer')

    # bench_run_span
    await cur.execute(
        'ALTER TABLE "bench_run_span" ADD COLUMN "severity" smallint NOT NULL DEFAULT 3'
    )

    # bench_run_plan
    await cur.execute(
        """
    CREATE TABLE "bench_run_plan" (
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
        "execution" smallint NOT NULL,
        "on_terminate" smallint NOT NULL,
        "on_error" smallint NOT NULL,
        "status" smallint NOT NULL DEFAULT 1,
        "duration" interval,
        "started_at" timestamp,
        "terminated_at" timestamp,
        "error" jsonb,
        "title" varchar,
        "text" jsonb,
        "calls" jsonb[] NOT NULL,
        "runs_id" uuid[] NOT NULL,
        "runs_bench_id" uuid[] NOT NULL,
        "runs_base_ck" uuid[],
        "runs_base_bench_id" uuid[],
        "step" integer,
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
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "severity" smallint NOT NULL')

    # bench_run_plan
    await cur.execute(
        'CREATE INDEX "bench_run_plan_bench_idx_created_at" ON bench_run_plan USING BTREE (created_at)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
