# This migration was automatically generated on 2024.03.13. Edit as needed.
import psycopg

ID = 3
VERSION = "2024.03.13.1"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_block
    await cur.execute(
        """
        ALTER TABLE bench_block    
        ALTER COLUMN order_key SET NOT NULL,
    ALTER COLUMN order_key SET DEFAULT 'a0'::character varying
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ALTER COLUMN order_key SET NOT NULL,
    ALTER COLUMN order_key SET DEFAULT 'a0'::character varying
    """
    )

    # bench_query
    await cur.execute(
        """
        ALTER TABLE bench_query    
        ALTER COLUMN order_key SET NOT NULL,
    ALTER COLUMN order_key SET DEFAULT 'a0'::character varying
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE bench_view    
        ALTER COLUMN order_key SET NOT NULL,
    ALTER COLUMN order_key SET DEFAULT 'a0'::character varying
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
