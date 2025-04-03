# This migration was automatically generated on 2025.04.03. Edit as needed.
import psycopg

ID = 42
VERSION = "2025.04.03.1"
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
        ADD COLUMN "computer_ck" uuid
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "on_terminate"
    """
    )

    # bench_scaler
    await cur.execute(
        """
        ALTER TABLE "bench_scaler"    
        ADD COLUMN "ck" uuid,
        ADD COLUMN "scaler_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_scaler"
        SET "ck" = "id",
            "scaler_ck" = "scaler_id"
        """
    )
    await cur.execute(
        """
        ALTER TABLE "bench_scaler"
        ALTER COLUMN "ck" SET NOT NULL
        """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        ADD COLUMN "ck" uuid,
        ADD COLUMN "scaler_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_store"
        SET "ck" = "id",
            "scaler_ck" = "scaler_id"
        """
    )
    await cur.execute(
        """
        ALTER TABLE "bench_store"
        ALTER COLUMN "ck" SET NOT NULL
        """
    )

    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        ADD COLUMN "ck" uuid,
        ADD COLUMN "scaler_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_computer"
        SET "ck" = "id",
            "scaler_ck" = "scaler_id"
        """
    )
    await cur.execute(
        """
        ALTER TABLE "bench_computer"
        ALTER COLUMN "ck" SET NOT NULL
        """
    )

    # bench_application
    await cur.execute(
        """
        ALTER TABLE "bench_application"    
        ADD COLUMN "ck" uuid,
        ADD COLUMN "scaler_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_application"
        SET "ck" = "id",
            "scaler_ck" = "scaler_id"
        """
    )
    await cur.execute(
        """
        ALTER TABLE "bench_application"
        ALTER COLUMN "ck" SET NOT NULL
        """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        ADD COLUMN "ck" uuid,
        ADD COLUMN "scaler_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_file"
        SET "ck" = "id",
            "scaler_ck" = "scaler_id"
        """
    )
    await cur.execute(
        """
        ALTER TABLE "bench_file"
        ALTER COLUMN "ck" SET NOT NULL
        """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE "bench_stream"    
        ADD COLUMN "ck" uuid,
        ADD COLUMN "scaler_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_stream"
        SET "ck" = "id",
            "scaler_ck" = "scaler_id"
        """
    )
    await cur.execute(
        """
        ALTER TABLE "bench_stream"
        ALTER COLUMN "ck" SET NOT NULL
        """
    )

    # bench_secret
    await cur.execute(
        """
        ALTER TABLE "bench_secret"    
        ADD COLUMN "ck" uuid,
        ADD COLUMN "scaler_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_secret"
        SET "ck" = "id",
            "scaler_ck" = "scaler_id"
        """
    )
    await cur.execute(
        """
        ALTER TABLE "bench_secret"
        ALTER COLUMN "ck" SET NOT NULL
        """
    )

    # bench_choice
    await cur.execute(
        """
        ALTER TABLE "bench_choice"    
        ADD COLUMN "parent_type" smallint
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "computer_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_thread"
        SET "computer_ck" = "computer_id"
        """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        ADD COLUMN "computer_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_agent"
        SET "computer_ck" = "computer_id"
        """
    )

    # bench_session
    await cur.execute(
        """
        ALTER TABLE "bench_session"    
        ADD COLUMN "computer_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_session"
        SET "computer_ck" = "computer_id"
        """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "computer_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_run"
        SET "computer_ck" = "computer_id"
        WHERE "computer_id" IS NOT NULL
        """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        ADD COLUMN "computer_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_span"
        SET "computer_ck" = "computer_id"
        WHERE "computer_id" IS NOT NULL
        """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        ADD COLUMN "computer_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_interruption"
        SET "computer_ck" = "computer_id"
        WHERE "computer_id" IS NOT NULL
        """
    )

    # bench_log
    await cur.execute(
        """
        ALTER TABLE "bench_log"    
        ADD COLUMN "computer_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_log"
        SET "computer_ck" = "computer_id"
        WHERE "computer_id" IS NOT NULL
        """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        ADD COLUMN "computer_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_plan"
        SET "computer_ck" = "computer_id"
        WHERE "computer_id" IS NOT NULL
        """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "computer_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_task"
        SET "computer_ck" = "computer_id"
        WHERE "computer_id" IS NOT NULL
        """
    )

    # bench_claim
    await cur.execute(
        """
        ALTER TABLE "bench_claim"    
        ADD COLUMN "computer_ck" uuid
    """
    )
    await cur.execute(
        """
        UPDATE "bench_claim"
        SET "computer_ck" = "computer_id"
        WHERE "computer_id" IS NOT NULL
        """
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
