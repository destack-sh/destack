# This migration was automatically generated on 2024.07.12. Edit as needed.
import psycopg

ID = 10
VERSION = "2024.07.11.7"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_file
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "kind" smallint NOT NULL')
    await cur.execute(
        """
        ALTER TABLE bench_file    
        ALTER COLUMN sha512 DROP NOT NULL,
        ALTER COLUMN retention DROP NOT NULL
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "node_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "node_base_ck"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "node_bench_id"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "node_ck"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "node_id"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "node_type"')
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "node_ptr" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
