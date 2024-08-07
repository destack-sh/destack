# This migration was automatically generated on 2024.08.07. Edit as needed.
import psycopg

ID = 34
VERSION = "2024.08.07.1"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_server
    await cur.execute("UPDATE bench_server SET current_status = 10")
    await cur.execute(
        """
        ALTER TABLE bench_server    
        ALTER COLUMN status DROP DEFAULT,
        ALTER COLUMN current_status SET NOT NULL,
        ALTER COLUMN version SET NOT NULL
    """
    )

    # bench_store
    await cur.execute("UPDATE bench_store SET current_status = 10")
    await cur.execute(
        """
        ALTER TABLE bench_store    
        ALTER COLUMN status DROP DEFAULT,
        ALTER COLUMN current_status SET NOT NULL,
        ALTER COLUMN version SET NOT NULL
    """
    )

    # bench_machine
    await cur.execute("UPDATE bench_machine SET current_status = 10")
    await cur.execute(
        """
        ALTER TABLE bench_machine    
        ALTER COLUMN status DROP DEFAULT,
        ALTER COLUMN current_status SET NOT NULL,
        ALTER COLUMN version SET NOT NULL
    """
    )

    # bench_drive
    await cur.execute("UPDATE bench_drive SET current_status = 10")
    await cur.execute(
        """
        ALTER TABLE bench_drive    
        ALTER COLUMN status DROP DEFAULT,
        ALTER COLUMN current_status SET NOT NULL
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
