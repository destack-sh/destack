# This migration was automatically generated on 2025.04.17. Edit as needed.
import psycopg

ID = 2
VERSION = "2025.04.17.4"
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
        ADD COLUMN "failed_at" timestamp,
        ADD COLUMN "failed_attempts" integer NOT NULL DEFAULT 0
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        ADD COLUMN "failed_at" timestamp,
        ADD COLUMN "failed_attempts" integer NOT NULL DEFAULT 0
    """
    )

    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        ADD COLUMN "failed_at" timestamp,
        ADD COLUMN "failed_attempts" integer NOT NULL DEFAULT 0
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        ADD COLUMN "failed_at" timestamp,
        ADD COLUMN "failed_attempts" integer NOT NULL DEFAULT 0
    """
    )

    # bench_stream
    await cur.execute(
        """
        ALTER TABLE "bench_stream"    
        ADD COLUMN "failed_at" timestamp,
        ADD COLUMN "failed_attempts" integer NOT NULL DEFAULT 0
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE "bench_link"    
        ADD COLUMN "failed_at" timestamp,
        ADD COLUMN "failed_attempts" integer NOT NULL DEFAULT 0
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
