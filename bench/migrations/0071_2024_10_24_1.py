# This migration was automatically generated on 2024.10.24. Edit as needed.
import psycopg

ID = 71
VERSION = "2024.10.24.1"
HAS_GLOBAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "text"')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "code"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "node_bench_id"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "node_ck"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "node_id"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "node_type"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "text"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "value_packed"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "value_type"')

    # bench_view
    await cur.execute(
        """
        ALTER TABLE bench_view    
        ALTER COLUMN "subviews_packed" DROP NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
