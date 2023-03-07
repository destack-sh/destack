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

        wire_module: wire.ModuleData = read_module(project_v)
        lang_module = wire.wmap_module(wire_module)
        print(render(lang_module.files))
