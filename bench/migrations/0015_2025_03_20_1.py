# This migration was automatically generated on 2025.03.20. Edit as needed.
import psycopg

ID = 15
VERSION = "2025.03.20.1"
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
    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        DROP COLUMN "identity_bench_id",
        DROP COLUMN "identity_ck",
        DROP COLUMN "identity_id",
        DROP COLUMN "roles_bench_id",
        DROP COLUMN "roles_id"
    """
    )

    # bench_identity
    await cur.execute(
        """
        ALTER TABLE "bench_identity"    
        DROP COLUMN "flow_bench_id",
        DROP COLUMN "flow_id"
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE "bench_flow"    
        ADD COLUMN "default_identity_id" uuid,
        ADD COLUMN "default_identity_ck" uuid,
        ADD COLUMN "default_identity_bench_id" uuid
    """
    )

    # bench_identity
    await cur.execute(
        """
        ALTER TABLE "bench_identity"    
        ADD COLUMN "default_flow_id" uuid,
        ADD COLUMN "default_flow_bench_id" uuid
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
