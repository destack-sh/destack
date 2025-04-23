# This migration was automatically generated on 2025.04.23. Edit as needed.
import psycopg

ID = 4
VERSION = "2025.04.23.0"
HAS_GLOBAL = False
HAS_REGIONAL = True
HAS_LOCAL = False


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
    # bench_field
    await cur.execute(
        """
        ALTER TABLE "bench_field"    
        ADD COLUMN "base_type_base_id" uuid
    """
    )

    # bench_option
    await cur.execute(
        """
        ALTER TABLE "bench_option"    
        ADD COLUMN "parent_type" smallint
    """
    )


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
