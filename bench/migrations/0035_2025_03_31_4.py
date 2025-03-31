# This migration was automatically generated on 2025.03.31. Edit as needed.
import psycopg

ID = 35
VERSION = "2025.03.31.4"
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
    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        DROP COLUMN "default_identity_bench_id",
        DROP COLUMN "default_identity_ck",
        DROP COLUMN "default_identity_id"
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "identity_bench_id",
        DROP COLUMN "identity_ck",
        DROP COLUMN "identity_id"
    """
    )

    # bench_session
    await cur.execute(
        """
        ALTER TABLE "bench_session"    
        DROP COLUMN "identity_bench_id",
        DROP COLUMN "identity_ck",
        DROP COLUMN "identity_id"
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "identity_bench_id",
        DROP COLUMN "identity_ck",
        DROP COLUMN "identity_id"
    """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        DROP COLUMN "identity_bench_id",
        DROP COLUMN "identity_ck",
        DROP COLUMN "identity_id"
    """
    )

    # bench_identity
    await cur.execute('DROP TABLE "bench_identity"')

    # bench_log
    await cur.execute(
        """
        ALTER TABLE "bench_log"    
        DROP COLUMN "identity_bench_id",
        DROP COLUMN "identity_ck",
        DROP COLUMN "identity_id"
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        DROP COLUMN "identity_bench_id",
        DROP COLUMN "identity_ck",
        DROP COLUMN "identity_id"
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        DROP COLUMN "identity_bench_id",
        DROP COLUMN "identity_ck",
        DROP COLUMN "identity_id"
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        DROP COLUMN "identity_bench_id",
        DROP COLUMN "identity_ck",
        DROP COLUMN "identity_id"
    """
    )

    # bench_claim
    await cur.execute(
        """
        ALTER TABLE "bench_claim"    
        DROP COLUMN "identity_bench_id",
        DROP COLUMN "identity_ck",
        DROP COLUMN "identity_id"
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        ADD COLUMN "default_agent_id" uuid,
        ADD COLUMN "default_agent_ck" uuid,
        ADD COLUMN "default_agent_bench_id" uuid
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "agent_id" uuid,
        ADD COLUMN "agent_ck" uuid,
        ADD COLUMN "agent_bench_id" uuid
    """
    )

    # bench_agent
    await cur.execute(
        """
    CREATE TABLE "bench_agent" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "claimed_by_id" uuid,
        "claimed_by_ck" uuid,
        "mode" smallint NOT NULL DEFAULT 20,
        "subnode_packed" jsonb,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "thread_ck" uuid,
        "tags_id" uuid[],
        "main_flow_id" uuid,
        "main_flow_bench_id" uuid,
        "implemented_by_id" uuid,
        "implemented_by_bench_id" uuid,
        "implemented_by_base_id" uuid,
        "color" smallint,
        "inputs_packed" jsonb,
        "outputs_packed" jsonb,
        "text" jsonb
    )
    """
    )

    # bench_session
    await cur.execute(
        """
        ALTER TABLE "bench_session"    
        ADD COLUMN "agent_id" uuid,
        ADD COLUMN "agent_ck" uuid,
        ADD COLUMN "agent_bench_id" uuid
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "agent_id" uuid,
        ADD COLUMN "agent_ck" uuid,
        ADD COLUMN "agent_bench_id" uuid
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        ADD COLUMN "agent_id" uuid,
        ADD COLUMN "agent_ck" uuid,
        ADD COLUMN "agent_bench_id" uuid
    """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        ADD COLUMN "agent_id" uuid,
        ADD COLUMN "agent_ck" uuid,
        ADD COLUMN "agent_bench_id" uuid
    """
    )

    # bench_log
    await cur.execute(
        """
        ALTER TABLE "bench_log"    
        ADD COLUMN "agent_id" uuid,
        ADD COLUMN "agent_ck" uuid,
        ADD COLUMN "agent_bench_id" uuid
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        ADD COLUMN "agent_id" uuid,
        ADD COLUMN "agent_ck" uuid,
        ADD COLUMN "agent_bench_id" uuid
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "agent_id" uuid,
        ADD COLUMN "agent_ck" uuid,
        ADD COLUMN "agent_bench_id" uuid
    """
    )

    # bench_claim
    await cur.execute(
        """
        ALTER TABLE "bench_claim"    
        ADD COLUMN "agent_id" uuid,
        ADD COLUMN "agent_ck" uuid,
        ADD COLUMN "agent_bench_id" uuid
    """
    )

    # bench_agent
    await cur.execute(
        'CREATE INDEX "bench_agent_bench_idx_parent_id" ON "bench_agent" USING BTREE (parent_id) INCLUDE (id)'
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
