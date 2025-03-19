# This migration was automatically generated on 2025.03.19. Edit as needed.
import psycopg

ID = 11
VERSION = "2025.03.19.0"
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
    # bench_membership
    await cur.execute(
        """
        ALTER TABLE "bench_membership"    
        DROP COLUMN "type"
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "identity_id" uuid,
        ADD COLUMN "identity_ck" uuid,
        ADD COLUMN "identity_bench_id" uuid
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE "bench_role"    
        ADD COLUMN "name" varchar
    """
    )

    # bench_identity
    await cur.execute(
        """
    CREATE TABLE "bench_identity" (
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
        "mode" smallint NOT NULL DEFAULT 20,
        "subnode_packed" jsonb,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "thread_ck" uuid,
        "tags_id" uuid[],
        "color" smallint,
        "flow_id" uuid,
        "flow_bench_id" uuid,
        "run_id" uuid,
        "run_bench_id" uuid,
        "run_base_id" uuid,
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
        ADD COLUMN "identity_id" uuid,
        ADD COLUMN "identity_ck" uuid,
        ADD COLUMN "identity_bench_id" uuid
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "identity_id" uuid,
        ADD COLUMN "identity_ck" uuid,
        ADD COLUMN "identity_bench_id" uuid
    """
    )

    # bench_run_span
    await cur.execute(
        """
        ALTER TABLE "bench_run_span"    
        ADD COLUMN "identity_id" uuid,
        ADD COLUMN "identity_ck" uuid,
        ADD COLUMN "identity_bench_id" uuid
    """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        ADD COLUMN "identity_id" uuid,
        ADD COLUMN "identity_ck" uuid,
        ADD COLUMN "identity_bench_id" uuid
    """
    )

    # bench_log
    await cur.execute(
        """
        ALTER TABLE "bench_log"    
        ADD COLUMN "identity_id" uuid,
        ADD COLUMN "identity_ck" uuid,
        ADD COLUMN "identity_bench_id" uuid
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        ADD COLUMN "identity_id" uuid,
        ADD COLUMN "identity_ck" uuid,
        ADD COLUMN "identity_bench_id" uuid
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "identity_id" uuid,
        ADD COLUMN "identity_ck" uuid,
        ADD COLUMN "identity_bench_id" uuid
    """
    )

    # bench_claim
    await cur.execute(
        """
        ALTER TABLE "bench_claim"    
        ADD COLUMN "identity_id" uuid,
        ADD COLUMN "identity_ck" uuid,
        ADD COLUMN "identity_bench_id" uuid
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ALTER COLUMN "type" SET DEFAULT 10
    """
    )

    # bench_membership
    await cur.execute(
        """
        ALTER TABLE "bench_membership"    
        ALTER COLUMN "bench_id" SET NOT NULL
    """
    )

    # bench_identity
    await cur.execute(
        'CREATE INDEX "bench_identity_bench_idx_parent_id" ON "bench_identity" USING BTREE (parent_id) INCLUDE (id)'
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
