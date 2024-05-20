# This migration was automatically generated on 2024.05.20. Edit as needed.
import psycopg

ID = 5
VERSION = "2024.05.20.2"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "bloom"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "uuid-ossp"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "vector"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "uuid-ossp"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "pgcrypto"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "timescaledb"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "bloom"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "pg_trgm"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
