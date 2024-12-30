# This migration was automatically generated on 2024.12.30. Edit as needed.
import psycopg

ID = 16
VERSION = "2024.12.30.4"
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
    # bench_field
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "partial_scope"')

    # bench_interruption
    await cur.execute('DROP INDEX "bench_interrupt_bench_idx_created_at"')
    await cur.execute('DROP INDEX "bench_interrupt_bench_idx_created_epoch"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "property_field_type" smallint')

    # bench_interruption
    await cur.execute(
        'CREATE INDEX "bench_interruption_bench_idx_created_at" ON bench_interruption USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_interruption_bench_idx_created_epoch" ON bench_interruption USING BTREE (created_epoch)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
