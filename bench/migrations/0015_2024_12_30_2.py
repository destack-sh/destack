# This migration was automatically generated on 2024.12.30. Edit as needed.
import psycopg

ID = 15
VERSION = "2024.12.30.2"
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
    # rename column in bench_run
    await cur.execute('ALTER TABLE "bench_run" RENAME COLUMN "interrupt_id" TO "interruption_id"')

    # rename table bench_interrupt to bench_interruption
    await cur.execute('ALTER TABLE "bench_interrupt" RENAME TO "bench_interruption"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
