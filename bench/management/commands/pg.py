import structlog
from asgiref.sync import async_to_sync
from django.core.management.base import BaseCommand
from django.db import transaction

from bench import models
from bench.models import Bench
from bench.sql.engine import create_local_pg_database, delete_local_pg_database, update_pg_schema

logger = structlog.get_logger(__name__)


async def update_pg_schema_from_db(bench_v: models.BenchVersion) -> None:
    from bench.runtime.utils import interp_module

    logger.info("pg.update_mappings", bench_version=repr(bench_v))
    module, bench = await interp_module(bench_v.id)
    await update_pg_schema(bench.pg_name, module)


class Command(BaseCommand):
    help = "Manage user PG databases."

    def add_arguments(self, parser):
        parser.add_argument("action", type=str)
        # optional arguments
        parser.add_argument("slug", type=str, nargs="?")

    def get_bench(self, slug: str) -> Bench:
        owner, bench_name = slug.split("/")
        bench = Bench.objects.get_by_slug(owner, bench_name)
        for attr in ("user", "organization", "head", "pg_username", "pg_password"):
            getattr(bench, attr)
        return bench

    @transaction.atomic
    def handle(self, action: str, slug: str | None, *args, **options):
        if slug == "all":
            benches = (
                models.Bench.objects.select_related("user", "organization", "head")
                .defer(None)
                .all()
            )
        elif slug:
            benches = [self.get_bench(slug)]
        else:
            benches = []
        if action == "bootstrap":
            pass
        elif action == "create":
            for bench in benches:
                async_to_sync(create_local_pg_database)(bench, upsert=True)
        elif action == "schema":
            for bench in benches:
                bench_v = bench.head
                bench_v.bench = bench  # 'preloaded' bench
                bench_v.bench.owner  # noqa why do we need to load this again?
                async_to_sync(update_pg_schema_from_db)(bench_v)
        elif action == "delete":
            for bench in benches:
                async_to_sync(delete_local_pg_database)(bench)
        else:
            raise ValueError("Unknown action")
