# This migration was automatically generated on 2024.08.06. Edit as needed.
import psycopg

ID = 32
VERSION = "2024.08.06.1"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_machine
    await cur.execute(
        'ALTER TABLE "bench_machine" ADD COLUMN "client_id" uuid REFERENCES bench_client ON DELETE SET NULL'
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
