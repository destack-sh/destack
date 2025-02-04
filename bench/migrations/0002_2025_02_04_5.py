# This migration was automatically generated on 2025.02.04. Edit as needed.
import psycopg

ID = 2
VERSION = "2025.02.04.5"
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
    # bench_class
    await cur.execute('ALTER TABLE "bench_class" DROP COLUMN "order_key"')

    # bench_flow
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "order_key"')

    # bench_choice
    await cur.execute('ALTER TABLE "bench_choice" DROP COLUMN "order_key"')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "order_key"')

    # bench_database
    await cur.execute('ALTER TABLE "bench_database" DROP COLUMN "order_key"')

    # bench_role
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "order_key"')

    # bench_identity
    await cur.execute('ALTER TABLE "bench_identity" DROP COLUMN "order_key"')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "text" jsonb')
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "block_id" uuid')
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "block_ck" uuid')
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "block_bench_id" uuid')

    # bench_choice
    await cur.execute(
        """
        ALTER TABLE bench_choice    
        ALTER COLUMN "block_id" DROP NOT NULL,
        ALTER COLUMN "block_ck" DROP NOT NULL,
        ALTER COLUMN "block_bench_id" DROP NOT NULL
    """
    )

    # bench_class
    await cur.execute(
        """
        ALTER TABLE bench_class    
        ALTER COLUMN "block_id" DROP NOT NULL,
        ALTER COLUMN "block_ck" DROP NOT NULL,
        ALTER COLUMN "block_bench_id" DROP NOT NULL
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE bench_flow    
        ALTER COLUMN "block_id" DROP NOT NULL,
        ALTER COLUMN "block_ck" DROP NOT NULL,
        ALTER COLUMN "block_bench_id" DROP NOT NULL
    """
    )

    # bench_database
    await cur.execute(
        """
        ALTER TABLE bench_database    
        ALTER COLUMN "block_id" DROP NOT NULL,
        ALTER COLUMN "block_ck" DROP NOT NULL,
        ALTER COLUMN "block_bench_id" DROP NOT NULL
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE bench_role    
        ALTER COLUMN "block_id" DROP NOT NULL,
        ALTER COLUMN "block_ck" DROP NOT NULL,
        ALTER COLUMN "block_bench_id" DROP NOT NULL
    """
    )

    # bench_identity
    await cur.execute(
        """
        ALTER TABLE bench_identity    
        ALTER COLUMN "block_id" DROP NOT NULL,
        ALTER COLUMN "block_ck" DROP NOT NULL,
        ALTER COLUMN "block_bench_id" DROP NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
