# This migration was automatically generated on 2024.08.14. Edit as needed.
import psycopg

ID = 38
VERSION = "2024.08.14.0"
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
    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "is_visible"')
    await cur.execute(
        'ALTER TABLE "bench_view" ADD COLUMN "is_hidden" boolean NOT NULL DEFAULT false'
    )
    await cur.execute(
        "UPDATE bench_view SET is_disabled = false, is_input = false, is_inline = false, is_loading = false"
    )
    await cur.execute(
        """
        ALTER TABLE bench_view    
        ALTER COLUMN is_disabled SET NOT NULL,
        ALTER COLUMN is_input SET NOT NULL,
        ALTER COLUMN is_inline SET NOT NULL,
        ALTER COLUMN is_loading SET NOT NULL
    """
    )

    # bench_run
    await cur.execute("UPDATE bench_run SET spans = spans || ARRAY[]::jsonb[]")
    await cur.execute("UPDATE bench_run SET events = events || ARRAY[]::jsonb[]")
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ALTER COLUMN spans SET NOT NULL,
        ALTER COLUMN events SET NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
