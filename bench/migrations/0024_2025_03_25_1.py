# This migration was automatically generated on 2025.03.25. Edit as needed.
import psycopg

ID = 24
VERSION = "2025.03.25.1"
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
    # bench_claim
    await cur.execute(
        """
        ALTER TABLE "bench_claim"    
        ADD COLUMN "target_template_id" uuid,
        ADD COLUMN "target_template_ck" uuid,
        ADD COLUMN "target_template_type" smallint,
        ADD COLUMN "target_template_bench_id" uuid
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
