# This migration was automatically generated on 2025.01.30. Edit as needed.
import psycopg

ID = 15
VERSION = "2025.01.30.1"
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
    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "processed_at" timestamp')
    await cur.execute(
        'ALTER TABLE "bench_trigger" ADD COLUMN "processed_count" integer NOT NULL DEFAULT 0'
    )
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "processed_key" varchar')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "trigger_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "trigger_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "trigger_key" varchar')
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_trigger_id_trigger_key" ON bench_run USING BTREE (trigger_id, trigger_key)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
