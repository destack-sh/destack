# This migration was automatically generated on 2024.02.14. Edit as needed.
import psycopg

ID = 3
VERSION = "2024.02.14.0"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_tag
    await cur.execute("DROP TABLE bench_tag")

    # bench_skip
    await cur.execute(
        """
        ALTER TABLE bench_skip    
        DROP COLUMN reference_tag_ck
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE bench_link    
        DROP COLUMN reference_tag_ck,
        DROP COLUMN value
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ADD COLUMN default_packed jsonb
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE bench_link    
        ADD COLUMN computed_reference jsonb
    """
    )

    # bench_block
    await cur.execute(
        """
        ALTER TABLE bench_block    
        ALTER COLUMN paused_at SET DATA TYPE timestamp WITH TIME ZONE
        USING to_timestamp(paused_at);
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
