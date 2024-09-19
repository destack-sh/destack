# This migration was automatically generated on 2024.09.19. Edit as needed.
import psycopg

ID = 48
VERSION = "2024.09.19.3"
HAS_GLOBAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


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
        ALTER TABLE bench_field    
        ALTER COLUMN is_secret SET NOT NULL,
    ALTER COLUMN is_secret SET DEFAULT false
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE bench_file    
        ALTER COLUMN retention SET NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
