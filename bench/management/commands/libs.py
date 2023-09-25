import os
import subprocess
from pathlib import Path
from uuid import UUID

import structlog
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench import models
from bench.language import wire
from bench.language.builtin import symbolx_lib
from bench.language.libs import DEFAULT_MODULES
from bench.language.mutate import diff_modules
from bench.models import packer
from bench.models.packer import DEFAULT_PACK_FILTER
from bench.utils.utils import DEBUG, LOCAL, TEST

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Manages generated Bench libraries"

    def add_arguments(self, parser: CommandParser):
        # action
        parser.add_argument("action", type=str, help="Action")
        # module name or 'all'
        parser.add_argument("module", type=str, help="Module")

    @transaction.atomic
    def handle(self, module: str, action: str, *args, **options):
        create_orgs_if_not_exist()

        if module == "all":
            modules = DEFAULT_MODULES.keys()
        else:
            modules = [module]

        if action == "upsert":
            for module in modules:
                _upsert_module(module, os.environ["VERSION"], sanity_check=DEBUG or TEST or LOCAL)
        elif action == "dump":
            for module in modules:
                _dump_module(module)
        else:
            raise ValueError(f"unknown action: {action}")


def create_orgs_if_not_exist():  # probably should put this elsewhere
    for org_name, org_slug in [
        ("SymbolX", "symbolx"),
        ("OpenAI", "openai"),
        ("Anthropic", "anthropic"),
    ]:
        if not models.Organization.objects.filter(owner_slug_id=org_slug).exists():
            models.Organization.objects.create_organization(name=org_name, slug=org_slug)
            logger.info("bootstrap_organization", organization=org_slug)


@transaction.atomic
def _upsert_module(module_name: str, version: str, sanity_check: bool):
    """
    Replace the module
    """
    log = logger.bind(module=module_name, version=version)
    log.info("lib.upsert")
    module = DEFAULT_MODULES[module_name]
    owner, name = module_name.split(".")
    try:
        project = models.Project.objects.get_by_slug(owner, name)

        # delete existing project version (with same tag) if it exists
        existing_project_v = project.versions.filter(tag=version).first()
        if existing_project_v is not None:
            log.info("lib.upsert.delete", project_v=existing_project_v)
            existing_project_v.delete()

        # create new project version
        project_v = models.ProjectVersion.objects.create(
            id=module.id, project=project, name=version, tag=version
        )
        project.head = project_v
        project.save()

        # link to parent
        parent = project.versions.filter(tag__lt=version).order_by("-tag").first()
        parents = [parent] if parent is not None else []
        project_v.parents.set(parents)
    except models.Project.DoesNotExist:
        # create new project
        owner = models.OwnerSlug.objects.get(slug=owner).owner
        project = models.Project.objects.create_project(
            id=module.ck,
            owner=owner,
            slug=name,
            name=name,
            visibility=models.ProjectVisibility.PUBLIC,
            head_version_id=module.id,
        )
        project_v = project.head
        project_v.tag = version
        project_v.name = version
        project_v.save()

    blank_module = packer.pack_module(project_v, filter=DEFAULT_PACK_FILTER)
    blank_module_tree = wire.ModuleTree(blank_module.nodes)
    new_module = wire.pack_module(module)
    mutations = diff_modules(blank_module, new_module)
    packer.write_mutations(project_v, blank_module_tree, mutations, refresh_index=False)
    project_v.commit()

    if sanity_check:
        # check: no issues after reload
        new_module_loaded_data = packer.pack_module(project_v, filter=DEFAULT_PACK_FILTER)
        new_module_loaded = wire.unpack_module(new_module_loaded_data, session=None)
        if name != "symbolx.lib":
            new_module_loaded.add_dependency(symbolx_lib)
        new_module_loaded.index()
        new_module_loaded._interp()
        if new_module_loaded.issues:
            raise ValueError(f"module {new_module_loaded} has issues: {new_module_loaded.issues}")

        # check: no diff when generated in another process
        _sanity_check_diff(module_name, new_module, log)

    log.info("lib.upsert.done", nodes=len(new_module.nodes))


def _sanity_check_diff(
    module_name: str, new_module: wire.ModuleTreeData, log: structlog.BoundLogger
) -> None:
    # start a new process, dump module, check if equal
    log.info("lib.upsert.sanity_check")
    process = subprocess.Popen(
        "python manage.py libs dump".split() + [module_name],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    stdout, stderr = process.communicate()
    assert process.returncode == 0, f"dump failed: {stderr}"

    # load other module
    other_module_path = _get_module_dump_path(module_name)
    other_module_bytes = Path(other_module_path).read_bytes()
    other_module_data = wire.deserialize_module(other_module_bytes)
    diff = diff_modules(new_module, other_module_data)

    if diff:
        # get exact diff for debugging
        module_tree = wire.ModuleTree(new_module.nodes)
        other_module_tree = wire.ModuleTree(other_module_data.nodes)

        def _get_path(n_id: UUID) -> str:
            if n_id in module_tree.nodes:
                path = module_tree.path_of(module_tree.nodes[n_id])
            else:
                path = other_module_tree.path_of(other_module_tree.nodes[n_id])
            return ".".join(n.name for n in path)

        diff_str = "\n".join(f"{m.data.id} {_get_path(m.data.id)}: {m.type} {m.data}" for m in diff)
        raise ValueError(f"module {module_name} is not equal to dumped module:\n{diff_str}")
    else:
        log.info("lib.upsert.sanity_check.ok", bytes=len(other_module_bytes))


_LIB_DUMP_DIR = "/tmp/bench_libs"


def _get_module_dump_path(module_name: str):
    return os.path.join(_LIB_DUMP_DIR, f"{module_name}.bench")


def _dump_module(module_name: str):
    module = DEFAULT_MODULES[module_name]
    module_data = wire.pack_module(module)
    module_bytes = wire.serialize_module(module_data)
    module_path = _get_module_dump_path(module_name)
    os.makedirs(os.path.dirname(module_path), exist_ok=True)
    Path(module_path).write_bytes(module_bytes)
