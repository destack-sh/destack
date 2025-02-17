# This migration was automatically generated on 2025.02.17. Edit as needed.
import psycopg

ID = 5
VERSION = "2025.02.17.1"
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
    # bench_run_plan
    await cur.execute('DROP TABLE "bench_run_plan"')

    # bench_plan
    await cur.execute(
        """
    CREATE TABLE "bench_plan" (
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
        "execution" smallint NOT NULL,
        "on_terminate" smallint NOT NULL,
        "on_error" smallint NOT NULL,
        "status" smallint NOT NULL DEFAULT 1,
        "started_by_id" uuid,
        "started_by_bench_id" uuid,
        "started_by_base_ck" uuid,
        "started_by_base_bench_id" uuid,
        "terminated_by_id" uuid,
        "terminated_by_bench_id" uuid,
        "terminated_by_base_ck" uuid,
        "terminated_by_base_bench_id" uuid,
        "title" varchar,
        "text" jsonb,
        "calls" jsonb[] NOT NULL,
        "error" jsonb,
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

    # bench_field
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ALTER COLUMN "bench_type" SET DATA TYPE integer
    """
    )

    # bench_plan
    await cur.execute(
        'CREATE INDEX "bench_plan_bench_idx_created_at" ON bench_plan USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_plan_bench_idx_parent_id" ON bench_plan USING BTREE (parent_id) INCLUDE (id)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
