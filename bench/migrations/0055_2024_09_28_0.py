# This migration was automatically generated on 2024.09.28. Edit as needed.
import psycopg

ID = 55
VERSION = "2024.09.28.0"
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
    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "bases_bench_id"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "bases_ck"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "bases_id"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "value_packed"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "value_packed"')

    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "filter_type"')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "variables_packed" jsonb')

    # bench_step
    await cur.execute(
        'ALTER TABLE "bench_step" ADD COLUMN "combinator" smallint NOT NULL DEFAULT 2'
    )

    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "filter" smallint')
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "constraint" jsonb')
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "condition" jsonb')
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "mapping" smallint')
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "modulation" smallint')
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "delay" interval')
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "size" integer')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "variables_packed" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
