# This migration was automatically generated on 2025.03.20. Edit as needed.
import psycopg

ID = 16
VERSION = "2025.03.20.2"
HAS_GLOBAL = False
HAS_REGIONAL = True
HAS_LOCAL = False


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
    # bench_run_span
    await cur.execute('DROP TABLE "bench_run_span"')

    # bench_span
    await cur.execute(
        """
    CREATE TABLE "bench_span" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 20,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "root_id" uuid,
        "root_base_id" uuid,
        "severity" smallint NOT NULL DEFAULT 3,
        "status" smallint NOT NULL DEFAULT 2,
        "duration" interval,
        "started_at" timestamp,
        "terminated_at" timestamp,
        "interrupted_at" timestamp,
        "interruption_id" uuid,
        "error" jsonb,
        "title" varchar,
        "text" jsonb,
        "code" jsonb,
        "nodes_id" uuid[],
        "nodes_ck" uuid[],
        "nodes_type" smallint[],
        "nodes_bench_id" uuid[],
        "nodes_base_id" uuid[],
        "flow_id" uuid,
        "kit_id" uuid,
        "action_id" uuid,
        "action_ck" uuid,
        "link_id" uuid,
        "plan_id" uuid,
        "plan_ck" uuid,
        "task_id" uuid,
        "task_ck" uuid,
        "trigger_id" uuid,
        "trigger_key" varchar,
        "message_id" uuid,
        "message_base_id" uuid,
        "session_id" uuid,
        "client_id" uuid,
        "computer_id" uuid,
        "user_id" uuid,
        "identity_id" uuid,
        "identity_ck" uuid,
        "identity_bench_id" uuid
    )
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "stopped_at" timestamp
    """
    )

    # bench_span
    await cur.execute(
        'CREATE INDEX "bench_span_bench_idx_created_at" ON "bench_span" USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_span_bench_idx_parent_id" ON "bench_span" USING BTREE (parent_id) INCLUDE (id)'
    )


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
