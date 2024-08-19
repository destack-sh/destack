# This migration was automatically generated on 2024.08.19. Edit as needed.
import psycopg

ID = 39
VERSION = "2024.08.19.2"
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
    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "condition"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "incoming_pipes"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "oneof_id" uuid')
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "oneof_ck" uuid')
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "oneof_type" smallint')
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "oneof_base_ck" uuid')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "pipes" jsonb[] NOT NULL')
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "ports" jsonb[] NOT NULL')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
