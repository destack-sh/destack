# This migration was automatically generated on 2024.12.17. Edit as needed.
import psycopg

ID = 5
VERSION = "2024.12.17.2"
HAS_GLOBAL = False
HAS_REGIONAL = True
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
    # bench_server
    await cur.execute(
        """
        ALTER TABLE bench_server    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE bench_store    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_drive
    await cur.execute(
        """
        ALTER TABLE bench_drive    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_vault
    await cur.execute(
        """
        ALTER TABLE bench_vault    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_cache
    await cur.execute(
        """
        ALTER TABLE bench_cache    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_machine
    await cur.execute(
        """
        ALTER TABLE bench_machine    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_browser
    await cur.execute(
        """
        ALTER TABLE bench_browser    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE bench_file    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE bench_stream    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_secret
    await cur.execute(
        """
        ALTER TABLE bench_secret    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_dependency
    await cur.execute(
        """
        ALTER TABLE bench_dependency    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE bench_space    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_block
    await cur.execute(
        """
        ALTER TABLE bench_block    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE bench_trigger    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_query
    await cur.execute(
        """
        ALTER TABLE bench_query    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE bench_view    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_step
    await cur.execute(
        """
        ALTER TABLE bench_step    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_pipe
    await cur.execute(
        """
        ALTER TABLE bench_pipe    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_session
    await cur.execute(
        """
        ALTER TABLE bench_session    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_interrupt
    await cur.execute(
        """
        ALTER TABLE bench_interrupt    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )

    # bench_log
    await cur.execute(
        """
        ALTER TABLE bench_log    
        ALTER COLUMN "mode" SET DEFAULT 2
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
