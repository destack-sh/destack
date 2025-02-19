# This migration was automatically generated on 2025.02.19. Edit as needed.
import psycopg

ID = 11
VERSION = "2025.02.19.4"
HAS_GLOBAL = False
HAS_REGIONAL = False
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
    pass


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_role
    await cur.execute(
        """
        ALTER TABLE bench_role    
        ALTER COLUMN "color" DROP DEFAULT
    """
    )

    # bench_identity
    await cur.execute(
        """
        ALTER TABLE bench_identity    
        ALTER COLUMN "color" DROP DEFAULT
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
