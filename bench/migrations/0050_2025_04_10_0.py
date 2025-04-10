# This migration was automatically generated on 2025.04.10. Edit as needed.
import psycopg

ID = 50
VERSION = "2025.04.10.0"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_client
    await cur.execute(
        """
        ALTER TABLE "bench_client"    
        DROP COLUMN "place_id"
    """
    )

    # bench_user
    await cur.execute(
        """
        ALTER TABLE "bench_user"    
        ADD COLUMN "main_cursor_id" uuid,
        ADD COLUMN "main_cursor_bench_id" uuid
    """
    )

    # bench_client
    await cur.execute(
        """
        ALTER TABLE "bench_client"    
        ADD COLUMN "main_cursor_id" uuid,
        ADD COLUMN "main_cursor_bench_id" uuid
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        ADD COLUMN "main_cursor_id" uuid
    """
    )

    # bench_cursor
    await cur.execute(
        """
    CREATE TABLE "bench_cursor" (
        "id" uuid NOT NULL PRIMARY KEY,
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
        "archived_at" timestamp,
        "deleted_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "mode" smallint NOT NULL DEFAULT 20,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "status" smallint NOT NULL DEFAULT 1,
        "active_at" timestamp,
        "target_id" uuid,
        "target_ck" uuid,
        "target_type" smallint,
        "target_bench_id" uuid,
        "target_base_id" uuid,
        "selection" jsonb,
        "focus" jsonb,
        "session_id" uuid,
        "client_id" uuid,
        "computer_id" uuid,
        "computer_ck" uuid,
        "user_id" uuid,
        "agent_id" uuid,
        "agent_ck" uuid,
        "agent_bench_id" uuid
    )
    """
    )
    await cur.execute(
        'CREATE INDEX "bench_cursor_bench_idx_parent_id" ON "bench_cursor" USING BTREE (parent_id) INCLUDE (id)'
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
