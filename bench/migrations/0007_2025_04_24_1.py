# This migration was automatically generated on 2025.04.24. Edit as needed.
import psycopg

ID = 7
VERSION = "2025.04.24.1"
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
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_handle
    await cur.execute(
        """
        ALTER TABLE "bench_handle"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_bench
    await cur.execute(
        """
        ALTER TABLE "bench_bench"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_user
    await cur.execute(
        """
        ALTER TABLE "bench_user"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_organization
    await cur.execute(
        """
        ALTER TABLE "bench_organization"    
        DROP COLUMN "subnode_packed"
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_scaler
    await cur.execute(
        """
        ALTER TABLE "bench_scaler"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_package
    await cur.execute(
        """
        ALTER TABLE "bench_package"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_dependency
    await cur.execute(
        """
        ALTER TABLE "bench_dependency"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_block
    await cur.execute(
        """
        ALTER TABLE "bench_block"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_choice
    await cur.execute(
        """
        ALTER TABLE "bench_choice"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE "bench_stream"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE "bench_link"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_class
    await cur.execute(
        """
        ALTER TABLE "bench_class"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_tag
    await cur.execute(
        """
        ALTER TABLE "bench_tag"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_kit
    await cur.execute(
        """
        ALTER TABLE "bench_kit"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_action
    await cur.execute(
        """
        ALTER TABLE "bench_action"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_transition
    await cur.execute(
        """
        ALTER TABLE "bench_transition"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE "bench_trigger"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_option
    await cur.execute(
        """
        ALTER TABLE "bench_option"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE "bench_field"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_database
    await cur.execute(
        """
        ALTER TABLE "bench_database"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        DROP COLUMN "plan_ck",
        DROP COLUMN "plan_id",
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE "bench_notification"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_team
    await cur.execute(
        """
        ALTER TABLE "bench_team"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_membership
    await cur.execute(
        """
        ALTER TABLE "bench_membership"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_invite
    await cur.execute(
        """
        ALTER TABLE "bench_invite"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE "bench_role"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_session
    await cur.execute(
        """
        ALTER TABLE "bench_session"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_log
    await cur.execute(
        """
        ALTER TABLE "bench_log"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_claim
    await cur.execute(
        """
        ALTER TABLE "bench_claim"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_cursor
    await cur.execute(
        """
        ALTER TABLE "bench_cursor"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE "bench_view"    
        DROP COLUMN "subnode_packed",
        DROP COLUMN "subviews_packed"
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE "bench_space"    
        DROP COLUMN "subnode_packed"
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        DROP COLUMN "subnode_packed"
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
