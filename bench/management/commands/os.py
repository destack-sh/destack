import asyncio

import structlog
from asgiref.sync import async_to_sync
from django.core.management.base import BaseCommand
from django.db import transaction

from bench import models
from bench.language.const import NodeType, StatementType
from bench.models import Bench, BenchVisibility
from bench.runtime.utils import interp_module
from bench.search.client import os_client_sync
from bench.search.engine import (
    create_global_os_index,
    create_global_os_role,
    create_local_os_index,
    enable_os_strict_mapping,
    sync_databases_to_os,
    update_os_schema,
)

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Manage OS indices."

    def add_arguments(self, parser):
        parser.add_argument("action", type=str)
        # optional arguments
        parser.add_argument("slug", type=str, nargs="?")

    def get_bench(self, slug: str) -> Bench:
        owner, bench_name = slug.split("/")
        bench = Bench.objects.get_by_slug(owner, bench_name)
        return bench

    @transaction.atomic
    def handle(self, action: str, slug: str | None, *args, **options):
        loop = asyncio.get_event_loop()
        if slug == "all":
            benches = models.Bench.objects.all()
        elif slug:
            benches = [self.get_bench(slug)]
        else:
            benches = []
        if action == "bootstrap":
            create_global_os_index(upsert=True)
            create_global_os_role()
        elif action == "create":
            for bench in benches:
                create_local_os_index(
                    os_name=bench.os_name,
                    os_username=bench.os_username,
                    os_password=bench.os_password,
                    is_public=bench.visibility == BenchVisibility.PUBLIC,
                    upsert=True,
                )
        elif action == "delete":
            for bench in benches:
                logger.info("opensearch.delete", bench=bench)
                os_client_sync.indices.delete(index=bench.os_name)
        elif action == "index":
            for bench in benches:
                # ignore fields not in mapping during reindex
                #  (fields may have existed in between snapshots)
                module, _ = async_to_sync(interp_module)(bench.head_id)
                loop.run_until_complete(update_os_schema(module, dynamic="false"))
                databases = [
                    n
                    for n in module._nodes
                    if n._type == NodeType.STATEMENT and n.type == StatementType.DATABASE
                ]
                loop.run_until_complete(sync_databases_to_os(module, databases))
                enable_os_strict_mapping(bench.os_name)
        else:
            raise ValueError("Unknown action")
