from __future__ import annotations

import datetime
from pathlib import Path

import structlog
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench import language
from bench.language import lex, parse, wire
from bench.language.lex import SourceFile
from bench.language.reconstruct import render
from bench.models import Organization, Project
from bench.models.mapper import lookup_in_db_module, read_module, write_module
from bench.models.project import ProjectVisibility

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Load Bench files into a project"

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
        lang_module = language.Module(id=project_v.id, name=project_v.project.path)
        source_file = SourceFile(path=path, content=Path(path).read_text())
        lang_module, _ = parse(lex(source_file), lang_module, lookup_in_db_module)
        wire_module = wire.rmap_module(lang_module)
        write_module(wire_module.files, project_v)

        # advance head to new version
        project.head = project_v
        project.save()

        # try to recover original source (sanity check)
        wire_module = read_module(project_v)
        lang_module = wire.wmap_module(wire_module)
        _ = render(lang_module.files)

        logger.info(f"Updated head to {project_v} in {project}")
