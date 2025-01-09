# This migration was automatically generated on 2025.01.09. Edit as needed.
import psycopg

ID = 27
VERSION = "2025.01.09.0"
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
    await cur.execute(
        'ALTER TABLE "bench_field" ALTER COLUMN "base_field_type" TYPE smallint[] USING CASE WHEN base_field_type IS NULL THEN ARRAY[]::smallint[] ELSE ARRAY[base_field_type] END'
    )
    await cur.execute(
        'ALTER TABLE "bench_field" ALTER COLUMN "property_field_type" TYPE smallint[] USING CASE WHEN property_field_type IS NULL THEN ARRAY[]::smallint[] ELSE ARRAY[property_field_type] END'
    )
    await cur.execute(
        'ALTER TABLE "bench_field" RENAME COLUMN "base_field_type" TO "base_field_types"'
    )
    await cur.execute(
        'ALTER TABLE "bench_field" RENAME COLUMN "property_field_type" TO "property_field_types"'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
