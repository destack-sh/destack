import asyncio
from pathlib import Path

import structlog
from django.core.management import BaseCommand, CommandParser
from django.db import transaction
from psycopg import sql

from bench import language as lang
from bench import models
from bench.language.const import NodeType
from bench.models import packer
from bench.models.utils import create_models_bfs
from bench.proto import wire
from bench.search.engine import update_os_schema
from bench.sql.client import async_pg_cursor
from bench.sql.engine import (
    PostgresConditionalOp,
    SqlComparison,
    pg_delete,
    pg_insert,
    pg_pack_record_row,
    update_pg_schema,
)
from bench.utils.func import partition
from bench.utils.utils import DEBUG, LOCAL

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Edit, dump and load Bench modules"

    def add_arguments(self, parser: CommandParser) -> None:
        parser.add_argument("action", type=str, help="Action")
        parser.add_argument("module", type=str, help="Module")
        # optional path
        parser.add_argument("path", type=str, nargs="?", help="Path")
        # optional history flag
        parser.add_argument("--after", type=str, help="After tag")
        # optional force flag
        parser.add_argument("--force", action="store_true", help="Force")
        # optional alias string
        parser.add_argument("--alias", type=str, help="Alias")
        # optional create flag
        parser.add_argument("--create", action="store_true", help="Create")

    async def _collect_local_records(
        self, module: lang.Module, *, versioned_only: bool
    ) -> list[wire.RecordData]:
        all_records: list[wire.RecordData] = []
        async with async_pg_cursor(module.pg_name) as cur:
            for database in module._nodes:
                if (
                    lang.HasDatabase not in database._components
                    or versioned_only
                    and not database.versioned
                ):
                    continue
                fetched = await database.records.first(2048)._do_fetch(cur)
                all_records.extend(fetched.records)
        return all_records

    async def _write_local_records(
        self, module: lang.Module, all_records_data: list[wire.RecordData]
    ):
        await update_pg_schema(module.pg_name, module)
        async with async_pg_cursor(module.pg_name) as cur:
            for database in module._nodes:
                if lang.HasDatabase not in database._components:
                    continue
                records_data = [r for r in all_records_data if r.parent_id == database.id]
                if records_data:
                    where = SqlComparison(
                        sql.Identifier("statement_key"), PostgresConditionalOp.EQ, database.key
                    )
                    await pg_delete(cur, database._table, where=where)
                    records_rows = [pg_pack_record_row(database, record) for record in records_data]
                    await pg_insert(cur, database._table, records_rows)

    @transaction.atomic
    def handle(
        self,
        module: str,
        action: str,
        path: str = None,
        after: str = None,
        force: bool = None,
        alias: str = None,
        create: bool = None,
        **options,
    ):
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        owner_slug, bench_slug = module.split("/")
        try:
            bench = models.Bench.objects.get_by_slug(owner_slug, bench_slug)
        except models.Bench.DoesNotExist:
            if create:
                owner = models.OwnerSlug.objects.get(slug=owner_slug).owner
                bench = models.Bench.objects.create_bench(
                    owner=owner,
                    name=bench_slug,
                    slug=bench_slug,
                    visibility=models.BenchVisibility.PRIVATE,
                )
            else:
                raise
        if path is None:
            path = "/tmp/bench/" + (alias or bench.path)

        logger.info(action, bench=bench, path=path)

        if action in ():
            pass
        else:
            raise ValueError(f"unknown action: {action}")
        loop.close()
