# This migration was automatically generated on 2024.12.18. Edit as needed.
import psycopg

ID = 6
VERSION = "2024.12.18.2"
HAS_GLOBAL = False
HAS_REGIONAL = True
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
    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "external_id" varchar')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "connection_uri" bytea')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "debugger_uri" bytea')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "view_uri" bytea')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "client_id" uuid')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "size" jsonb')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "target_size" jsonb')

    # bench_machine
    await cur.execute(
        """
        ALTER TABLE bench_machine    
        ALTER COLUMN "type" DROP DEFAULT
    """
    )


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "font"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "variant"')
    await cur.execute(
        'ALTER TABLE "bench_view" ADD COLUMN "is_minimal" boolean NOT NULL DEFAULT false'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
