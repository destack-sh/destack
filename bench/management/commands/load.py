from __future__ import annotations

from pathlib import Path

import structlog
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench import language
from bench.language import SymbolType, lex, parse, wire
from bench.language.lex import SourceFile
from bench.language.reconstruct import render
from bench.models import OwnerSlug, Project
from bench.models.mapper import lookup_in_db_module, read_module, write_module
from bench.models.project import ProjectVisibility

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Load Bench files into a project"

    def add_arguments(self, parser: CommandParser):
        # project as organization/project
        parser.add_argument("owner_project", type=str)
        # symbol file path (must exist and end in .py)
        parser.add_argument("path", type=str)

    @transaction.atomic
    def handle(self, owner_project: str, path: str, *args, **options):
        owner_slug, project_slug = owner_project.split("/")
        try:
            project = Project.objects.get_by_slug(owner_slug, project_slug)
        except Project.DoesNotExist:
            owner = OwnerSlug.objects.get(slug=owner_slug).owner
            project = Project.objects.create_project(
                owner=owner,
                name=project_slug,
                slug=project_slug,
                visibility=ProjectVisibility.PRIVATE,
                create_onboarding_files=False,
            )

        project_v = project.create_version(name="Update from CLI")
        project_v.reset()

        if not Path(path).is_dir():
            source_files = [SourceFile(path=path, content=Path(path).read_text())]
        else:
            # if it's a directory, load all files
            source_files = []
            for file in Path(path).glob("**/*.bench"):
                source_files.append(SourceFile(path=file, content=file.read_text()))

        lang_module = language.Module(id=project_v.id, name=project_v.project.path)
        for source_file in source_files:
            _ = parse(lex(source_file), lang_module, lookup_in_db_module)

        # :ManageRequirements
        # strip requirements from module because we can't manage them interactively set
        # we obviously shouldn't do this later on (hopefully soon..)
        for file in lang_module.files:
            file.statements = [
                s for s in file.statements if s.symbol_type != SymbolType.REQUIREMENT
            ]

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
