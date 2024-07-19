# This migration was automatically generated on 2024.07.19. Edit as needed.
import psycopg

ID = 14
VERSION = "2024.07.19.0"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_file
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "coarse_type" smallint NOT NULL')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "width" integer')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "height" integer')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "aspect_ratio" real')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "codec" varchar')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "duration" real')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "bitrate" integer')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "channels" integer')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "sample_rate" integer')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "visibility"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "format_hint"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "visibility"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
