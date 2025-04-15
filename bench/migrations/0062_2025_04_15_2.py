# This migration was automatically generated on 2025.04.15. Edit as needed.
import psycopg

ID = 62
VERSION = "2025.04.15.2"
HAS_GLOBAL = False
HAS_REGIONAL = True
HAS_LOCAL = False


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
    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        DROP COLUMN "external_url"
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE "bench_link"    
        DROP COLUMN "image_bench_id",
        DROP COLUMN "image_ck",
        DROP COLUMN "image_id",
        DROP COLUMN "image_url"
    """
    )

    # bench_file
    await cur.execute(
        """
        ALTER TABLE "bench_file"    
        ADD COLUMN "url" varchar,
        ADD COLUMN "content_url" varchar,
        ADD COLUMN "thumbnail_url" varchar,
        ADD COLUMN "favicon_url" varchar,
        ADD COLUMN "thumbnail_width" integer,
        ADD COLUMN "thumbnail_height" integer
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE "bench_link"    
        ADD COLUMN "content_url" varchar,
        ADD COLUMN "thumbnail_url" varchar,
        ADD COLUMN "favicon_url" varchar,
        ADD COLUMN "thumbnail_width" integer,
        ADD COLUMN "thumbnail_height" integer,
        ADD COLUMN "attribution" varchar,
        ADD COLUMN "published_at" timestamp
    """
    )


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
