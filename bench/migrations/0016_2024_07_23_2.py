# This migration was automatically generated on 2024.07.23. Edit as needed.
import psycopg

ID = 16
VERSION = "2024.07.23.2"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_file
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_file_bench_idx_parent_id_sha256" ON bench_file USING BTREE (parent_id, sha256)'
    )
    await cur.execute(
        'ALTER TABLE "bench_file" ADD CONSTRAINT "bench_file_bench_idx_parent_id_sha256" UNIQUE USING INDEX bench_file_bench_idx_parent_id_sha256'
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
