import psycopg

# <Metadata>
ID = "<ID>"
COMMIT = "<COMMIT>"
VERSION = "<VERSION>"
HAS_GLOBAL = "<HAS_GLOBAL>"
HAS_LOCAL = "<HAS_LOCAL>"
# </Metadata>

#
# Global DB for core Bench nodes (n=1)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    pass


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (n=|Benches|)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    pass
