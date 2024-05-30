# This migration was automatically generated on 2024.05.30. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.05.30.0"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_server
    await cur.execute(
        """
        ALTER TABLE bench_server    
        ALTER COLUMN region DROP DEFAULT
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE bench_store    
        ALTER COLUMN region DROP DEFAULT
    """
    )

    # bench_machine
    await cur.execute(
        """
        ALTER TABLE bench_machine    
        ALTER COLUMN region DROP DEFAULT
    """
    )

    # bench_drive
    await cur.execute(
        """
        ALTER TABLE bench_drive    
        ALTER COLUMN region DROP DEFAULT
    """
    )

    # bench_blob
    await cur.execute(
        """
        ALTER TABLE bench_blob    
        ALTER COLUMN region DROP DEFAULT
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
