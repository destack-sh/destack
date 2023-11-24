from pathlib import Path

import structlog
from django.core.management.base import BaseCommand
from django.db import transaction

from bench.models import Project

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Local dev stuff"

    def add_arguments(self, parser):
        parser.add_argument("action", choices=["worker-imitate"])
        # optional slug
        parser.add_argument("slug", nargs="?")

    def get_project(self, slug: str) -> Project:
        owner, project_name = slug.split("/")
        project = Project.objects.get_by_slug(owner, project_name)
        return project

    @transaction.atomic
    def handle(self, action: str, slug: str | None, *args, **options):
        if slug == "all":
            projects = Project.objects.all()
        elif slug:
            projects = [self.get_project(slug)]
        else:
            projects = []

        if action == "worker-imitate":
            # write worker env vars to .env.worker
            if projects:
                assert len(projects) == 1, "only one project supported"
                project = projects[0]
                env_vars = {
                    "WORKER_SET_ID": str(project.worker_set.id),
                    "WORKER_NODE_ID": "local",
                    "WORKER_PROJECT_ID": str(project.id),
                    "WORKER_MODULE_ID": str(project.head_id),
                    "LOCAL_PG_NAME": project.pg_name,
                    "LOCAL_PG_USERNAME": project.pg_username,
                    "LOCAL_PG_PASSWORD": project.pg_password,
                    "LOCAL_OS_NAME": project.os_name,
                    "LOCAL_OS_USERNAME": project.os_username,
                    "LOCAL_OS_PASSWORD": project.os_password,
                }
                Path(".env.worker").write_text("\n".join(f"{k}={v}" for k, v in env_vars.items()))
                self.stdout.write(self.style.SUCCESS(f"patched .env.worker for {project}"))
            else:
                # truncate .env.worker
                Path(".env.worker").write_text("")
                self.stdout.write(self.style.SUCCESS("cleared .env.worker"))
            # restart worker (touch manageworker.py)
            Path("manageworker.py").touch()
        else:
            raise ValueError(f"unknown action: {action}")
