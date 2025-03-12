# This migration was automatically generated on 2025.03.12. Edit as needed.
import psycopg

ID = 8
VERSION = "2025.03.12.2"
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
    # bench_dependency
    await cur.execute(
        """
        ALTER TABLE "bench_dependency"    
        DROP COLUMN "depends_on_bench_id",
        DROP COLUMN "depends_on_packages_bench_id",
        DROP COLUMN "depends_on_packages_id"
    """
    )

    # bench_package
    await cur.execute(
        """
        ALTER TABLE "bench_package"    
        DROP COLUMN "base_id",
        DROP COLUMN "text"
    """
    )

    # bench_scaler
    await cur.execute(
        """
        ALTER TABLE "bench_scaler"    
        DROP COLUMN "is_main"
    """
    )

    # bench_dependency
    await cur.execute(
        """
        ALTER TABLE "bench_dependency"    
        ADD COLUMN "dependency_id" uuid NOT NULL,
        ADD COLUMN "dependency_bench_id" uuid NOT NULL
    """
    )

    # bench_package
    await cur.execute(
        """
        ALTER TABLE "bench_package"    
        ALTER COLUMN "slug" DROP NOT NULL
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
