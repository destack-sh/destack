import asyncio
from pathlib import Path

import structlog
from django.core.management import BaseCommand, CommandParser
from django.db import transaction
from psycopg import sql

from bench import language as lang
from bench import models
from bench.language import wire
from bench.language.const import NodeType
from bench.models import packer
from bench.models.utils import create_models_bfs
from bench.search.engine import update_os_schema
from bench.sql.client import async_pg_cursor
from bench.sql.engine import (
    PostgresConditionalOp,
    SqlComparison,
    pg_delete,
    pg_insert,
    pg_pack_record_row,
    update_pg_schema,
)
from bench.utils.func import partition
from bench.utils.utils import DEBUG, LOCAL

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Edit, dump and load Bench modules"

    def add_arguments(self, parser: CommandParser) -> None:
        parser.add_argument("action", type=str, help="Action")
        parser.add_argument("module", type=str, help="Module")
        # optional path
        parser.add_argument("path", type=str, nargs="?", help="Path")
        # optional history flag
        parser.add_argument("--after", type=str, help="After tag")
        # optional force flag
        parser.add_argument("--force", action="store_true", help="Force")
        # optional alias string
        parser.add_argument("--alias", type=str, help="Alias")
        # optional create flag
        parser.add_argument("--create", action="store_true", help="Create")

    async def _collect_local_records(
        self, module: lang.Module, *, versioned_only: bool
    ) -> list[wire.RecordData]:
        all_records: list[wire.RecordData] = []
        async with async_pg_cursor(module.pg_name) as cur:
            for database in module._nodes:
                if (
                    lang.HasDatabase not in database._components
                    or versioned_only
                    and not database.versioned
                ):
                    continue
                fetched = await database.records.first(2048)._do_fetch(cur)
                all_records.extend(fetched.records)
        return all_records

    async def _write_local_records(
        self, module: lang.Module, all_records_data: list[wire.RecordData]
    ):
        await update_pg_schema(module.pg_name, module)
        async with async_pg_cursor(module.pg_name) as cur:
            for database in module._nodes:
                if lang.HasDatabase not in database._components:
                    continue
                records_data = [r for r in all_records_data if r.parent_id == database.id]
                if records_data:
                    where = SqlComparison(
                        sql.Identifier("statement_key"), PostgresConditionalOp.EQ, database.key
                    )
                    await pg_delete(cur, database._table, where=where)
                    records_rows = [pg_pack_record_row(database, record) for record in records_data]
                    await pg_insert(cur, database._table, records_rows)

    @transaction.atomic
    def handle(
        self,
        module: str,
        action: str,
        path: str = None,
        after: str = None,
        force: bool = None,
        alias: str = None,
        create: bool = None,
        **options,
    ):
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        owner_slug, project_slug = module.split("/")
        try:
            project = models.Project.objects.get_by_slug(owner_slug, project_slug)
        except models.Project.DoesNotExist:
            if create:
                owner = models.OwnerSlug.objects.get(slug=owner_slug).owner
                project = models.Project.objects.create_project(
                    owner=owner,
                    name=project_slug,
                    slug=project_slug,
                    visibility=models.ProjectVisibility.PRIVATE,
                )
            else:
                raise
        if path is None:
            path = "/tmp/bench/" + (alias or project.path)

        logger.info(action, project=project, path=path)

        if action in ("paste", "splice"):
            project_v = project.head
            if path == "-":
                files = list(project_v.files.all())
            else:
                try:
                    files = [project_v.files.get(name=path)]
                except models.File.DoesNotExist:
                    raise ValueError(
                        f"file '{path}' not found in {project_v}, others: {list(project_v.files.all().values_list('name', flat=True))}"
                    )
            models.ProjectVersion.objects.copy(
                source=project_v,
                target=project_v,
                files=files,
                keep_cks=False,
                include_interp=False,
            )
            if action == "splice":
                # delete the original files
                models.File.objects.filter(id__in=[f.id for f in files]).delete()
            logger.info(action, files=files)
        elif action == "dump":
            assert path is not None, "path is required for dump"

            # wipe directory
            for file in Path(path).glob("*.bench"):
                file.unlink()
            Path(path).mkdir(parents=True, exist_ok=True)

            # dump filtered versions
            versions = project.versions.order_by("-tag").filter(tag__gte=after or "0")
            for version in list(versions) + [project.head]:
                # host/inline data
                module_data = packer.pack_module_host(version, excluded=[])
                # add local records
                module = lang.Module.make(
                    module_data.nodes,
                    project_id=project.id,
                    os_name=project.os_name,
                    pg_name=project.pg_name,
                )
                if version.id == project.head_id:  # only the head version has all tables
                    records_data = loop.run_until_complete(
                        self._collect_local_records(module, versioned_only=True)
                    )
                    module_data.nodes.extend(records_data)
                else:
                    records_data = ()
                # serialize & write
                module_bytes = wire.serialize_module(module_data)
                tag_clean = version.tag.replace(".", "-") if version.tag else "head"
                module_path = path + "/" + tag_clean + ".bench"
                Path(module_path).write_bytes(module_bytes)
                logger.info(
                    "dump",
                    version=version,
                    path=module_path,
                    nodes=len(module_data.nodes),
                    records=len(records_data),
                    bytes=len(module_bytes),
                )
        elif action == "load":
            assert path is not None, "path is required for load"

            paths = list(Path(path).glob("*.bench"))
            paths = sorted(paths, key=lambda p: p.stem)
            if not paths:
                logger.info(
                    "load.skip",
                    reason="no files found",
                    path=path,
                    glob=list(Path(path).glob("*.bench")),
                )
                return

            existing_versions = project.versions.order_by("-tag").filter(tag__gte=after or "0")
            existing_versions = list(existing_versions) + [project.head]

            for module_path in paths:
                tag = module_path.stem.replace("-", ".") if module_path.stem != "head" else None
                module_bytes = Path(module_path).read_bytes()
                module_data = wire.deserialize_module(module_bytes)

                project_v = next((v for v in existing_versions if v.tag == tag), None)
                if not force and project_v and project_v.tag is not None:
                    # skip existing versions (but allow head)
                    logger.info("load.skip", version=project_v, path=module_path)
                    continue
                if project_v:
                    project_v.delete()

                project_v = models.ProjectVersion.objects.create(
                    project=project,
                    ck=project.id,
                    id=module_data.module.id,
                    tag=tag,
                    name=tag,
                    committed_at=module_data.module.updated_at if tag else None,
                )

                # wipe project version
                host_nodes, record_nodes = partition(
                    lambda n: n._type == NodeType.RECORD, module_data.nodes
                )
                logger.info(
                    "load",
                    version=project_v,
                    path=module_path,
                    nodes=len(module_data.nodes),
                    records=len(record_nodes),
                    bytes=len(module_bytes),
                )
                unpacked = packer.unpack_nodes_tree(
                    host_nodes, pre_unpacked={project_v.id: project_v}
                )
                create_models_bfs(unpacked.walk_bfs_batched(), exclude=[project_v.id])
                # write records to local
                module = lang.Module.make(
                    module_data.nodes,
                    project_id=project.id,
                    os_name=project.os_name,
                    pg_name=project.pg_name,
                )
                if project.head_id == project_v.id:
                    # write head to OS
                    loop.run_until_complete(update_os_schema(module))
                if record_nodes:
                    loop.run_until_complete(self._write_local_records(module, record_nodes))
            # set parents to previous version
            for version in project.versions.exclude(tag=None).order_by("-tag"):
                if version.parents.exists():
                    break
                previous = project.versions.filter(tag__lt=version.tag).order_by("-tag").first()
                if previous:
                    version.parents.set([previous])
                    version.save()

            # reset head
            last_non_head = project.versions.exclude(tag=None).order_by("-tag").first()
            project.head = project.versions.filter(tag=None).get()
            project.head.parents.set([last_non_head] if last_non_head else [])
            project.save()

            if DEBUG or LOCAL:
                # touch file to restart any running process
                Path("manage.py").touch()
        else:
            raise ValueError(f"unknown action: {action}")
        loop.close()
