# This migration was automatically generated on 2024.10.19. Edit as needed.
import psycopg

ID = 69
VERSION = "2024.10.19.0"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_server
    await cur.execute(
        """
        ALTER TABLE bench_server    
        ALTER COLUMN min_cpu SET DATA TYPE real,
        ALTER COLUMN max_cpu SET DATA TYPE real,
        ALTER COLUMN min_ram SET DATA TYPE real,
        ALTER COLUMN max_ram SET DATA TYPE real
    """
    )

    # bench_machine
    await cur.execute(
        """
        ALTER TABLE bench_machine    
        ALTER COLUMN cpu SET DATA TYPE real,
        ALTER COLUMN current_cpu SET DATA TYPE real,
        ALTER COLUMN ram SET DATA TYPE real,
        ALTER COLUMN current_ram SET DATA TYPE real
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE bench_file    
        ALTER COLUMN aspect_ratio SET DATA TYPE real
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
