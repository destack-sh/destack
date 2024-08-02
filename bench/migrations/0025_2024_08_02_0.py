# This migration was automatically generated on 2024.08.02. Edit as needed.
import psycopg

ID = 25
VERSION = "2024.08.02.0"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_handle
    await cur.execute(
        """
        ALTER TABLE bench_handle    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_type DROP NOT NULL
    """
    )

    # bench_client
    await cur.execute(
        """
        ALTER TABLE bench_client    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_type DROP NOT NULL
    """
    )

    # bench_server
    await cur.execute(
        """
        ALTER TABLE bench_server    
        ALTER COLUMN parent_id DROP NOT NULL
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE bench_store    
        ALTER COLUMN parent_id DROP NOT NULL
    """
    )

    # bench_machine
    await cur.execute(
        """
        ALTER TABLE bench_machine    
        ALTER COLUMN parent_id DROP NOT NULL
    """
    )

    # bench_drive
    await cur.execute(
        """
        ALTER TABLE bench_drive    
        ALTER COLUMN parent_id DROP NOT NULL
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_branch
    await cur.execute(
        """
        ALTER TABLE bench_branch    
        ALTER COLUMN parent_id DROP NOT NULL
    """
    )

    # bench_package
    await cur.execute(
        """
        ALTER TABLE bench_package    
        ALTER COLUMN parent_id DROP NOT NULL
    """
    )

    # bench_dependency
    await cur.execute(
        """
        ALTER TABLE bench_dependency    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_ck DROP NOT NULL,
        ALTER COLUMN parent_type DROP NOT NULL
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE bench_space    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_ck DROP NOT NULL,
        ALTER COLUMN parent_type DROP NOT NULL
    """
    )

    # bench_block
    await cur.execute(
        """
        ALTER TABLE bench_block    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_ck DROP NOT NULL,
        ALTER COLUMN parent_type DROP NOT NULL
    """
    )

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE bench_trigger    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_ck DROP NOT NULL
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_ck DROP NOT NULL,
        ALTER COLUMN parent_type DROP NOT NULL
    """
    )

    # bench_query
    await cur.execute(
        """
        ALTER TABLE bench_query    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_ck DROP NOT NULL
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE bench_view    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_ck DROP NOT NULL,
        ALTER COLUMN parent_type DROP NOT NULL
    """
    )

    # bench_step
    await cur.execute(
        """
        ALTER TABLE bench_step    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_ck DROP NOT NULL,
        ALTER COLUMN parent_type DROP NOT NULL
    """
    )

    # bench_badge
    await cur.execute(
        """
        ALTER TABLE bench_badge    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_ck DROP NOT NULL,
        ALTER COLUMN parent_type DROP NOT NULL
    """
    )

    # bench_secret
    await cur.execute(
        """
        ALTER TABLE bench_secret    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_ck DROP NOT NULL,
        ALTER COLUMN parent_type DROP NOT NULL
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE bench_file    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_ck DROP NOT NULL,
        ALTER COLUMN parent_type DROP NOT NULL
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE bench_message    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_ck DROP NOT NULL,
        ALTER COLUMN parent_type DROP NOT NULL
    """
    )

    # bench_membership
    await cur.execute(
        """
        ALTER TABLE bench_membership    
        ALTER COLUMN parent_id DROP NOT NULL
    """
    )

    # bench_invite
    await cur.execute(
        """
        ALTER TABLE bench_invite    
        ALTER COLUMN parent_id DROP NOT NULL
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE bench_notification    
        ALTER COLUMN parent_id DROP NOT NULL
    """
    )

    # bench_session
    await cur.execute(
        """
        ALTER TABLE bench_session    
        ALTER COLUMN parent_id DROP NOT NULL
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ALTER COLUMN parent_id DROP NOT NULL,
        ALTER COLUMN parent_type DROP NOT NULL
    """
    )

    # bench_signal
    await cur.execute(
        """
        ALTER TABLE bench_signal    
        ALTER COLUMN parent_id DROP NOT NULL
    """
    )

    # bench_log
    await cur.execute(
        """
        ALTER TABLE bench_log    
        ALTER COLUMN parent_id DROP NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
