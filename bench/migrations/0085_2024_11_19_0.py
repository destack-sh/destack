# This migration was automatically generated on 2024.11.19. Edit as needed.
import psycopg

ID = 85
VERSION = "2024.11.19.0"
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
    # bench_interrupt
    await cur.execute('ALTER TABLE "bench_interrupt" ADD COLUMN "breakpoint_site" smallint')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
