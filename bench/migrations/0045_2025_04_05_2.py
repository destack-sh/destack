# This migration was automatically generated on 2025.04.05. Edit as needed.
import psycopg

ID = 45
VERSION = "2025.04.05.2"
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
    # bench_secret
    await cur.execute('DROP TABLE "bench_secret"')

    # bench_store
    await cur.execute(
        """
        ALTER TABLE "bench_store"    
        ALTER COLUMN "sql_url" SET DATA TYPE varchar
    """
    )

    # bench_computer
    await cur.execute(
        """
        ALTER TABLE "bench_computer"    
        ALTER COLUMN "grpc_url" SET DATA TYPE varchar
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
