from __future__ import annotations

import datetime
from pathlib import Path

import structlog
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.language import lex, parse
from bench.language.lex import SourceFile
from bench.models import Organization, Project
from bench.models.mapper import lookup_module_in_db, rmap_module, write
from bench.models.project import ProjectVisibility

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Loads a instructions from a file into a project"

    def add_arguments(self, parser: CommandParser):
        # project as organization/project
        parser.add_argument("organization_project", type=str)
        # symbol file path (must exist and end in .py)
        parser.add_argument("path", type=str)

    @transaction.atomic
    def handle(self, organization_project: str, path: str, *args, **options):
        organization_slug, project_slug = organization_project.split("/")
        organization = Organization.objects.get(slug=organization_slug)
        project = Project.objects.filter(slug=project_slug, organization=organization).first()
        if project is None:
            project = Project.objects.create_project(
                organization=organization,
                name=project_slug,
                slug=project_slug,
                visibility=ProjectVisibility.PRIVATE,
            )

        last_modified = datetime.datetime.fromtimestamp(Path(path).stat().st_mtime)
        version_id = str(int(last_modified.timestamp()))

        project_v = project.create_version(name=version_id)
        project_v.reset()
        module = rmap_module(project_v)
        source_file = SourceFile(path=path, content=Path(path).read_text())
        language_files = parse(lex(source_file), module, lookup_module_in_db).files
        write(language_files, project_v)

        # advance head to new version
        project.head = project_v
        project.save()

        logger.info(f"Updated head to {project_v} in {project}")
