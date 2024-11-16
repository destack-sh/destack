# This migration was automatically generated on 2024.11.16. Edit as needed.
import psycopg

ID = 82
VERSION = "2024.11.16.0"
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
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "run_options" jsonb')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "run_options" jsonb')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "incoming_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "incoming_base_ck" uuid[]')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "outgoing_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "outgoing_base_ck" uuid[]')

    # bench_interrupt
    await cur.execute('ALTER TABLE "bench_interrupt" ADD COLUMN "attempt_no" integer')
    await cur.execute('ALTER TABLE "bench_interrupt" ADD COLUMN "outputs_packed" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
