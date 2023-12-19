import asyncio

import structlog
from asgiref.sync import async_to_sync
from django.core.management.base import BaseCommand
from django.db import transaction

from bench import models
from bench.language.const import NodeType, StatementType
from bench.models import Project
from bench.search.client import os_client_sync
from bench.search.engine import update_os_schema
from bench.server.runtime import interp_module
from bench.server.search import (
    create_global_os_index,
    create_global_os_role,
    create_local_os_index,
    enable_os_strict_mapping,
    sync_databases_to_os,
)

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Manage OS indices."

    def add_arguments(self, parser):
        parser.add_argument("action", type=str)
        # optional arguments
        parser.add_argument("slug", type=str, nargs="?")

    def get_project(self, slug: str) -> Project:
        owner, project_name = slug.split("/")
        project = Project.objects.get_by_slug(owner, project_name)
        return project

    @transaction.atomic
    def handle(self, action: str, slug: str | None, *args, **options):
        loop = asyncio.get_event_loop()
        if slug == "all":
            projects = models.Project.objects.all()
        elif slug:
            projects = [self.get_project(slug)]
        else:
            projects = []
        if action == "bootstrap":
            create_global_os_index(upsert=True)
            create_global_os_role()
        elif action == "create":
            for project in projects:
                create_local_os_index(project, upsert=True)
        elif action == "delete":
            for project in projects:
                logger.info("opensearch.delete", project=project)
                os_client_sync.indices.delete(index=project.os_name)
        elif action == "index":
            for project in projects:
                # ignore fields not in mapping during reindex
                #  (fields may have existed in between snapshots)
                module, _ = async_to_sync(interp_module)(project.head_id)
                module._os_name = project.os_name
                module._pg_name = project.pg_name
                loop.run_until_complete(update_os_schema(module, dynamic="false"))
                databases = [
                    n
                    for n in module._nodes
                    if n._type == NodeType.STATEMENT and n.type == StatementType.DATABASE
                ]
                loop.run_until_complete(sync_databases_to_os(module, databases))
                enable_os_strict_mapping(project.os_name)
        else:
            raise ValueError("Unknown action")
