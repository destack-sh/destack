from django.core.management import BaseCommand

from bench.language import wire
from bench.language.reconstruct import render
from bench.models import Organization, Project
from bench.models.mapper import read_module


class Command(BaseCommand):
    help = "Dump a project version to a Bench file"

    def add_arguments(self, parser):
        # project as organization/project
        parser.add_argument("organization_project", type=str)
        # version id (optional, default to HEAD)
        parser.add_argument("-version", type=str, default="HEAD")

    def handle(self, organization_project, version, *args, **options):
        organization_slug, project_slug = organization_project.split("/")
        organization = Organization.objects.get(slug=organization_slug)
        project = Project.objects.filter(slug=project_slug, organization=organization).get()
        if version == "HEAD":
            project_v = project.head
        else:
            project_v = project.versions.get(name=version)

        wire_module: wire.ModuleData = read_module(project_v)
        lang_module = wire.wmap_module(wire_module)
        print(render(lang_module.files))
