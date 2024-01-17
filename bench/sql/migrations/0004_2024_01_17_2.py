# This migration was automatically generated on 2024.01.17. Edit as needed.
import psycopg

ID = 4
VERSION = "2024.01.17.2"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_bench
    await cur.execute(
        "CREATE UNIQUE INDEX bench_bench_bench_idx_slug ON bench_bench USING BTREE (slug)"
    )

    # bench_badge
    await cur.execute(
        "CREATE UNIQUE INDEX bench_badge_bench_idx_link_token ON bench_badge USING BTREE (link_token)"
    )

    # bench_handle
    await cur.execute(
        "CREATE UNIQUE INDEX bench_handle_bench_idx_slug ON bench_handle USING BTREE (slug)"
    )

    # bench_user
    await cur.execute(
        "CREATE UNIQUE INDEX bench_user_bench_idx_slug ON bench_user USING BTREE (slug)"
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_user_bench_idx_email ON bench_user USING BTREE (email)"
    )

    # bench_organization
    await cur.execute(
        "CREATE UNIQUE INDEX bench_organization_bench_idx_slug ON bench_organization USING BTREE (slug)"
    )

    # bench_client
    await cur.execute(
        "CREATE UNIQUE INDEX bench_client_bench_idx_access_token ON bench_client USING BTREE (access_token)"
    )

    # bench_worker
    await cur.execute(
        "CREATE UNIQUE INDEX bench_worker_bench_idx_external_id ON bench_worker USING BTREE (external_id)"
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError()


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError()
