# This migration was automatically generated on 2025.03.06. Edit as needed.
import psycopg

ID = 39
VERSION = "2025.03.06.2"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_bench
    await cur.execute(
        """
        ALTER TABLE bench_bench    
        ALTER COLUMN "owned_by_ck" DROP NOT NULL
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
        ALTER TABLE bench_scaler    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE bench_store    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_machine
    await cur.execute(
        """
        ALTER TABLE bench_machine    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_browser
    await cur.execute(
        """
        ALTER TABLE bench_browser    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE bench_file    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE bench_stream    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_secret
    await cur.execute(
        """
        ALTER TABLE bench_secret    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_package
    await cur.execute(
        """
        ALTER TABLE bench_package    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_page
    await cur.execute(
        """
        ALTER TABLE bench_page    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_choice
    await cur.execute(
        """
        ALTER TABLE bench_choice    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_class
    await cur.execute(
        """
        ALTER TABLE bench_class    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_option
    await cur.execute(
        """
        ALTER TABLE bench_option    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_tag
    await cur.execute(
        """
        ALTER TABLE bench_tag    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE bench_flow    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_action
    await cur.execute(
        """
        ALTER TABLE bench_action    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE bench_link    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE bench_trigger    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_kit
    await cur.execute(
        """
        ALTER TABLE bench_kit    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE bench_view    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_database
    await cur.execute(
        """
        ALTER TABLE bench_database    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE bench_channel    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE bench_role    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE bench_space    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE bench_plan    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE bench_task    
        ALTER COLUMN "name" DROP NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
