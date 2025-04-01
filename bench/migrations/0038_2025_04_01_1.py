# This migration was automatically generated on 2025.04.01. Edit as needed.
import psycopg

ID = 38
VERSION = "2025.04.01.1"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_bench
    await cur.execute(
        """
        ALTER TABLE "bench_bench"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_handle
    await cur.execute(
        """
        ALTER TABLE "bench_handle"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_user
    await cur.execute(
        """
        ALTER TABLE "bench_user"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_organization
    await cur.execute(
        """
        ALTER TABLE "bench_organization"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_client
    await cur.execute(
        """
        ALTER TABLE "bench_client"    
        ADD COLUMN "archived_at" timestamp
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
        DROP COLUMN "template_at"
    """
    )

    # bench_package
    await cur.execute(
        """
        ALTER TABLE "bench_package"    
        DROP COLUMN "template_at"
    """
    )

    # bench_dependency
    await cur.execute(
        """
        ALTER TABLE "bench_dependency"    
        DROP COLUMN "template_at"
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        DROP COLUMN "template_at"
    """
    )

    # bench_block
    await cur.execute(
        """
        ALTER TABLE "bench_block"    
        DROP COLUMN "template_at"
    """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE "bench_stream"    
        DROP COLUMN "template_at"
    """
    )

    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        DROP COLUMN "template_at"
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE "bench_field"    
        DROP COLUMN "template_at"
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        DROP COLUMN "template_at"
    """
    )

    # bench_action
    await cur.execute(
        """
        ALTER TABLE "bench_action"    
        DROP COLUMN "template_at"
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE "bench_link"    
        DROP COLUMN "template_at"
    """
    )

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE "bench_trigger"    
        DROP COLUMN "template_at"
    """
    )

    # bench_option
    await cur.execute(
        """
        ALTER TABLE "bench_option"    
        DROP COLUMN "template_at"
    """
    )

    # bench_choice
    await cur.execute(
        """
        ALTER TABLE "bench_choice"    
        DROP COLUMN "template_at"
    """
    )

    # bench_class
    await cur.execute(
        """
        ALTER TABLE "bench_class"    
        DROP COLUMN "template_at"
    """
    )

    # bench_tag
    await cur.execute(
        """
        ALTER TABLE "bench_tag"    
        DROP COLUMN "template_at"
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE "bench_space"    
        DROP COLUMN "template_at"
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE "bench_view"    
        DROP COLUMN "template_at"
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE "bench_role"    
        DROP COLUMN "template_at"
    """
    )

    # bench_kit
    await cur.execute(
        """
        ALTER TABLE "bench_kit"    
        DROP COLUMN "template_at"
    """
    )

    # bench_database
    await cur.execute(
        """
        ALTER TABLE "bench_database"    
        DROP COLUMN "template_at"
    """
    )

    # bench_membership
    await cur.execute(
        """
        ALTER TABLE "bench_membership"    
        DROP COLUMN "template_at"
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        DROP COLUMN "template_at"
    """
    )

    # bench_team
    await cur.execute(
        """
        ALTER TABLE "bench_team"    
        DROP COLUMN "template_at"
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        DROP COLUMN "template_at"
    """
    )

    # bench_secret
    await cur.execute(
        """
        ALTER TABLE "bench_secret"    
        DROP COLUMN "template_at"
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "template_at"
    """
    )

    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        DROP COLUMN "template_at"
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        DROP COLUMN "template_at"
    """
    )

    # bench_claim
    await cur.execute(
        """
        ALTER TABLE "bench_claim"    
        DROP COLUMN "template_at"
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        DROP COLUMN "template_at"
    """
    )

    # bench_application
    await cur.execute(
        """
        ALTER TABLE "bench_application"    
        DROP COLUMN "template_at"
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        DROP COLUMN "template_at"
    """
    )

    # bench_scaler
    await cur.execute(
        """
        ALTER TABLE "bench_scaler"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_application
    await cur.execute(
        """
        ALTER TABLE "bench_application"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE "bench_stream"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_secret
    await cur.execute(
        """
        ALTER TABLE "bench_secret"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_package
    await cur.execute(
        """
        ALTER TABLE "bench_package"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_dependency
    await cur.execute(
        """
        ALTER TABLE "bench_dependency"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_block
    await cur.execute(
        """
        ALTER TABLE "bench_block"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_choice
    await cur.execute(
        """
        ALTER TABLE "bench_choice"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_class
    await cur.execute(
        """
        ALTER TABLE "bench_class"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE "bench_field"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_option
    await cur.execute(
        """
        ALTER TABLE "bench_option"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_tag
    await cur.execute(
        """
        ALTER TABLE "bench_tag"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_action
    await cur.execute(
        """
        ALTER TABLE "bench_action"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE "bench_link"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE "bench_trigger"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_kit
    await cur.execute(
        """
        ALTER TABLE "bench_kit"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_database
    await cur.execute(
        """
        ALTER TABLE "bench_database"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE "bench_notification"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_team
    await cur.execute(
        """
        ALTER TABLE "bench_team"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_membership
    await cur.execute(
        """
        ALTER TABLE "bench_membership"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_invite
    await cur.execute(
        """
        ALTER TABLE "bench_invite"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE "bench_role"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_agent
    await cur.execute(
        """
        ALTER TABLE "bench_agent"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_session
    await cur.execute(
        """
        ALTER TABLE "bench_session"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_log
    await cur.execute(
        """
        ALTER TABLE "bench_log"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_claim
    await cur.execute(
        """
        ALTER TABLE "bench_claim"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE "bench_view"    
        ADD COLUMN "archived_at" timestamp
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE "bench_space"    
        ADD COLUMN "archived_at" timestamp
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
