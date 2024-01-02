from pathlib import Path

import structlog
from django.core.management.base import BaseCommand
from django.db import transaction

from bench.models import Bench

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Local dev stuff"

    def add_arguments(self, parser):
        parser.add_argument("action", choices=["worker-imitate"])
        # optional slug
        parser.add_argument("slug", nargs="?")

    def get_bench(self, slug: str) -> Bench:
        owner, bench_name = slug.split("/")
        bench = Bench.objects.get_by_slug(owner, bench_name)
        return bench

    @transaction.atomic
    def handle(self, action: str, slug: str | None, *args, **options):
        if slug == "all":
            benches = Bench.objects.all()
        elif slug:
            benches = [self.get_bench(slug)]
        else:
            benches = []

        if action == "worker-imitate":
            # write worker env vars to .env.worker
            if benches:
                assert len(benches) == 1, "only one bench supported"
                bench = benches[0]
                env_vars = {
                    "WORKER_SET_ID": str(bench.worker_set.id),
                    "WORKER_NODE_ID": "local",
                    "WORKER_BENCH_ID": str(bench.id),
                    "WORKER_MODULE_ID": str(bench.head_id),
                    "LOCAL_PG_NAME": bench.pg_name,
                    "LOCAL_PG_USERNAME": bench.pg_username,
                    "LOCAL_PG_PASSWORD": bench.pg_password,
                    "LOCAL_OS_NAME": bench.os_name,
                    "LOCAL_OS_USERNAME": bench.os_username,
                    "LOCAL_OS_PASSWORD": bench.os_password,
                }
                Path(".env.worker").write_text("\n".join(f"{k}={v}" for k, v in env_vars.items()))
                self.stdout.write(self.style.SUCCESS(f"patched .env.worker for {bench}"))
            else:
                # truncate .env.worker
                Path(".env.worker").write_text("")
                self.stdout.write(self.style.SUCCESS("cleared .env.worker"))
            # restart worker (touch manageworker.py)
            Path("manageworker.py").touch()
        else:
            raise ValueError(f"unknown action: {action}")
