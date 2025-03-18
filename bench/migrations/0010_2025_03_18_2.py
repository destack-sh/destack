# This migration was automatically generated on 2025.03.18. Edit as needed.
import psycopg

ID = 10
VERSION = "2025.03.18.2"
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
    # bench_scaler
    await cur.execute(
        """
        ALTER TABLE "bench_scaler"    
        DROP COLUMN "text"
    """
    )

    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        DROP COLUMN "text"
    """
    )

    # bench_browser
    await cur.execute(
        """
        ALTER TABLE "bench_browser"    
        DROP COLUMN "text"
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        DROP COLUMN "text"
    """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE "bench_stream"    
        DROP COLUMN "text"
    """
    )

    # bench_choice
    await cur.execute(
        """
        ALTER TABLE "bench_choice"    
        DROP COLUMN "text"
    """
    )

    # bench_class
    await cur.execute(
        """
        ALTER TABLE "bench_class"    
        DROP COLUMN "text"
    """
    )

    # bench_tag
    await cur.execute(
        """
        ALTER TABLE "bench_tag"    
        DROP COLUMN "text"
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE "bench_view"    
        DROP COLUMN "text"
    """
    )

    # bench_database
    await cur.execute(
        """
        ALTER TABLE "bench_database"    
        DROP COLUMN "text"
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE "bench_role"    
        DROP COLUMN "text"
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "text"
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        DROP COLUMN "text"
    """
    )

    # bench_secret
    await cur.execute(
        """
        ALTER TABLE "bench_secret"    
        DROP COLUMN "text"
    """
    )

    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        DROP COLUMN "text"
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        DROP COLUMN "text"
    """
    )

    # bench_team
    await cur.execute(
        """
        ALTER TABLE "bench_team"    
        DROP COLUMN "text"
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
