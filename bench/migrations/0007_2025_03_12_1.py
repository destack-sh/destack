# This migration was automatically generated on 2025.03.12. Edit as needed.
import psycopg

ID = 7
VERSION = "2025.03.12.1"
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

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "mode" smallint NOT NULL DEFAULT 2,
        ADD COLUMN "session_id" uuid,
        ADD COLUMN "client_id" uuid,
        ADD COLUMN "machine_id" uuid,
        ADD COLUMN "user_id" uuid
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        ADD COLUMN "mode" smallint NOT NULL DEFAULT 2
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE "bench_notification"    
        ADD COLUMN "mode" smallint NOT NULL DEFAULT 2
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
