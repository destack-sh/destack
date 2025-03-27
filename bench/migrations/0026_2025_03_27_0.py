# This migration was automatically generated on 2025.03.27. Edit as needed.
import psycopg

ID = 26
VERSION = "2025.03.27.0"
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
    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        ADD COLUMN "width" integer,
        ADD COLUMN "height" integer
        """
    )

    # Set initial values
    await cur.execute(
        """
        UPDATE "bench_computer"
        SET "width" = 1280, "height" = 960
        """
    )

    # Make columns NOT NULL
    await cur.execute(
        """
        ALTER TABLE "bench_computer"
        ALTER COLUMN "width" SET NOT NULL,
        ALTER COLUMN "height" SET NOT NULL
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
