# This migration was automatically generated on 2024.09.19. Edit as needed.
import psycopg

ID = 49
VERSION = "2024.09.19.4"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_user
    await cur.execute(
        """
        ALTER TABLE bench_user    
        ALTER COLUMN email DROP NOT NULL
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_space
    await cur.execute(
        """
        ALTER TABLE bench_space    
        ALTER COLUMN bar_position SET NOT NULL
    """
    )

    # bench_field
    await cur.execute(
        """
    UPDATE bench_field SET is_secret = false WHERE is_secret IS NULL
"""
    )
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ALTER COLUMN is_secret SET NOT NULL,
    ALTER COLUMN is_secret SET DEFAULT false
    """
    )

    # bench_file
    await cur.execute(
        """
        UPDATE bench_file SET retention = 1 WHERE retention IS NULL
    """
    )
    await cur.execute(
        """
        ALTER TABLE bench_file    
        ALTER COLUMN retention SET NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
