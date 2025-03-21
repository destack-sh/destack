# This migration was automatically generated on 2025.03.21. Edit as needed.
import psycopg

ID = 20
VERSION = "2025.03.21.2"
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
    # bench_invite
    await cur.execute(
        """
        ALTER TABLE "bench_invite"    
        DROP COLUMN "is_owner",
        DROP COLUMN "to_bench_id",
        DROP COLUMN "to_id",
        DROP COLUMN "to_type",
        DROP COLUMN "type",
        DROP COLUMN "user_email",
        DROP COLUMN "user_id",
        ADD COLUMN "member_id" uuid NOT NULL,
        ADD COLUMN "member_type" smallint NOT NULL,
        ADD COLUMN "member_bench_id" uuid
    """
    )

    # bench_membership
    await cur.execute(
        """
        ALTER TABLE "bench_membership"    
        ALTER COLUMN "member_bench_id" DROP NOT NULL
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
