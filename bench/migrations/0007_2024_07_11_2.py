# This migration was automatically generated on 2024.07.11. Edit as needed.
import psycopg

ID = 7
VERSION = "2024.07.11.2"
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
    # bench_record_shared
    await cur.execute('ALTER TABLE "bench_record_shared" DROP COLUMN "secret_value_packed"')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "secret_value_packed"')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "secret_value_packed"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "inputs_secret_packed"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "outputs_secret_packed"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "value_secret_packed"')

    # bench_signal
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "secret_value_packed"')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "secret_value_packed"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "secret_value_packed"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
