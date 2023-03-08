# supress all logging
import logging

from django.core.management import BaseCommand

from bench.language import wire
from bench.language.reconstruct import render
from bench.models import Project
from bench.models.mapper import read_module

logging.disable(logging.CRITICAL)


class Command(BaseCommand):
    help = "Dump a project version to a Bench file"

    def add_arguments(self, parser):
        # project as owner/project
        parser.add_argument("owner_project", type=str)
        # version tag (optional, default to x)
        parser.add_argument("-tag", type=str, default="x")

    def handle(self, owner_project, tag, *args, **options):
        owner_slug, project_slug = owner_project.split("/")
        project = Project.objects.get_by_slug(owner_slug, project_slug)
        if tag == "x":
            project_v = project.head
        else:
            project_v = project.versions.get(tag=tag)

        # we include implicit requirements here and strip them again after load :ManageRequirements
        # (because the requirements are needed to parse the module)
        wire_module: wire.ModuleData = read_module(project_v, add_implicit_requirements=True)
        lang_module = wire.wmap_module(wire_module)
        # exclude generated files (except implicit requirements)
        files = [f for f in lang_module.files if not f.generated or f.path == "__implicit__"]
        print(render(files))
