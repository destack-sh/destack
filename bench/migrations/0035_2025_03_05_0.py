# This migration was automatically generated on 2025.03.05. Edit as needed.
import psycopg

ID = 35
VERSION = "2025.03.05.0"
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
    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "failure_mode"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "termination_mode"')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "on_terminate" smallint NOT NULL')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "on_failure" smallint NOT NULL')

    # bench_field
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ALTER COLUMN "type" DROP DEFAULT
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
