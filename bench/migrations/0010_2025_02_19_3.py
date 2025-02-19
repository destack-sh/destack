# This migration was automatically generated on 2025.02.19. Edit as needed.
import psycopg

ID = 10
VERSION = "2025.02.19.3"
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
    # bench_option
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "base_field_types"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "base_type_bench_id"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "base_type_ck"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "base_type_id"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "base_type_type"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "bench_type"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "condition"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "constraint"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "default_packed"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "format"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "is_list"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "is_required"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "is_secret"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "kind"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "oneof_base_ck"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "oneof_ck"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "oneof_id"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "oneof_type"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "primitive_type"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "property_field_types"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
