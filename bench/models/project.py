from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING, Optional
from uuid import UUID

import pytz
from asgiref.sync import sync_to_async
from django.core.validators import validate_slug
from django.db import models, transaction
from django.db.models import Q, QuerySet
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.models.symbol import SymbolContent, SymbolDefinition, SymbolType
from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel

if TYPE_CHECKING:
    from bench.models.organization import Organization


class ProjectType(models.TextChoices):
    EXECUTABLE = "executable", "Executable"
    LIBRARY = "library", "Library"


class ProjectManager(models.Manager["Project"]):
    @transaction.atomic
    def create_project(
        self,
        organization: Organization,
        name: str,
        slug: str,
        type: ProjectType = ProjectType.EXECUTABLE,
    ):
        project = super().create(organization=organization, name=name, slug=slug, type=type)
        project.head = ProjectVersion.objects.create(project=project)
        project.save()
        return project

    def get_by_slug(self, organization: str, project: str):
        return self.get(organization__slug=organization, slug=project)


class Project(TaggableMixin, UUIDModel):
    """
    A project to instruct an AI to do something.

    Projects are the root of versioning, similar to repositories in Git.
    All versions are available in 'versions' and may not be linear (also like in Git).
    Projects define symbols organized into files.

    Executable projects have main programs (top-level program instructions).
    Library projects define reusable symbols (like in software).
    """

    type = TextChoicesField(choices_enum=ProjectType, default=ProjectType.EXECUTABLE)
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    slug: models.SlugField = models.SlugField(max_length=128, validators=[validate_slug])
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    # TODO @Feature: basic branching, move Project head into main_branch.head
    head = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, null=True, related_name="project+"
    )
    # branches via ProjectBranch
    # programs via Program

    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="projects"
    )

    def __str__(self):
        return f"{self.organization.slug}/{self.slug}"

    @property
    def head_(self) -> ProjectVersion:
        if self.head is None:
            raise ValueError(f"project {self} has no head")
        return self.head

    @transaction.atomic
    def create_version(
        self,
        name: Optional[str] = None,
        description: Optional[str] = None,
        parent: Optional[ProjectVersion] = None,
        auto_commit: bool = True,
        commit_name: Optional[str] = None,
    ) -> "ProjectVersion":
        if parent is None:
            if self.head is None:
                raise ValueError(f"project does not have a head version: {self}")
            assigned_parent = self.head
        else:
            assigned_parent = parent
        del parent  # avoid accidental use

        if not assigned_parent.committed:
            if auto_commit:
                assigned_parent.commit(commit_name)
            else:
                raise ValueError(f"parent version must be committed: {assigned_parent}")

        new_version = ProjectVersion.objects.create(
            project=self, name=name, description=description
        )
        new_version.parents.add(assigned_parent)

        # copy all project files and their symbol definitions from parent
        # TODO @Performance: copy project version on commit server-side in SQL
        #  This is awfully sequential and slow, particularly deepcopy of symbol definitions.
        # Also TODO @Cleanup: created_at/updated_at are not copied correctly (they are set to now)
        # 1. copy project files
        new_files: dict[UUID, File] = {}
        for file in assigned_parent.files.all():
            old_id = file.id
            file.pk = None
            file.project_version = new_version
            file.save()
            new_files[old_id] = file
        # 1.1 re-assign project file parents
        for old_file in assigned_parent.files.all():
            if old_file.parent_id is not None:
                new_file = new_files[old_file.id]
                new_file.parent = new_files[old_file.parent_id]
                new_file.save()
        # 2. copy symbol definitions and symbol contents
        new_definitions: dict[UUID, SymbolDefinition] = {}
        new_contents: dict[UUID, SymbolContent] = {}
        for definition in assigned_parent.definitions.all():
            old_content: SymbolContent = definition.content
            # copy content
            old_id = old_content.id
            content = old_content
            content.pk = None
            content.save()
            new_contents[old_id] = content
            # copy definition
            old_id = definition.id
            definition.pk = None
            definition.parent = None
            definition.set_content(content)
            definition.file = new_files[definition.file_id]
            definition.project_version = new_version
            definition.save()
            new_definitions[old_id] = definition
        refs: dict[UUID, SymbolContent | SymbolDefinition] = {**new_definitions, **new_contents}
        # 2.1 re-assign references and deep copy symbols
        for old_definition in assigned_parent.definitions.all():
            new_definition = new_definitions[old_definition.id]
            new_content = new_contents[old_definition.content_id]
            # copy content
            old_content = old_definition.content
            old_content.deepcopy(to=new_content, refs=refs)
            new_content.save()
            # re-assign definition parent and content
            if old_definition.parent_id is not None:
                new_definition.parent = new_definitions[old_definition.parent_id]
            new_definition.save()

        # head has advanced to new version
        if assigned_parent == self.head:
            self.head = new_version

        return new_version

    objects: ProjectManager = ProjectManager()

    class Meta:
        default_related_name = "projects"
        constraints = [
            models.UniqueConstraint(
                name="bench_project_organization_slug_ak",
                fields=["organization", "slug"],
            )
        ]


class ProjectBranch(UUIDModel):
    """
    A branch of a project, like in Git. A branch is a pointer to a project version.
    """

    project: models.ForeignKey = models.ForeignKey(
        "Project", on_delete=models.CASCADE, related_name="branches"
    )
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    head: models.ForeignKey = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="branches+"
    )

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_project_branch_project_name_ak",
                fields=["project", "name"],
            )
        ]


class ProjectVersion(TaggableMixin, UUIDModel):
    """
    A project version records the state of a project at a specific point in time.
    """

    project = models.ForeignKey(Project, on_delete=models.CASCADE, related_name="versions")
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    committed_at = models.DateTimeField(null=True)

    parents = models.ManyToManyField(
        "ProjectVersion", related_name="children", symmetrical=False, blank=True
    )
    libraries = models.ManyToManyField("ProjectVersion", related_name="dependents", blank=True)
    main_program = models.ForeignKey(
        "SymbolDefinition", related_name="+", null=True, on_delete=models.SET_NULL
    )
    # files via ProjectFile
    # definitions via SymbolDefinition
    backends: models.ManyToManyField = models.ManyToManyField(
        "SymbolDefinition", related_name="referenced_in_projects+", blank=True
    )

    def __str__(self) -> str:
        return f"{self.organization.slug}/{self.project.slug}@{self.id.hex}"

    def available_definitions(self, include_libraries: bool = True) -> QuerySet[SymbolDefinition]:
        # get own and libraries definitions (non-recursive for now)
        if include_libraries:
            return SymbolDefinition.objects.filter(
                Q(project_version_id__in=(self.id, *self.libraries.values_list("id", flat=True)))
            )
        else:
            return self.definitions.all()

    @transaction.atomic
    def create_file(
        self,
        name: str,
        parent: Optional[File] = None,
        definitions: Optional[list[SymbolDefinition]] = None,
    ) -> "File":
        file = File.objects.create(project_version=self, parent=parent, name=name)
        if definitions:
            file.definitions.set(definitions)
        return file

    @transaction.atomic
    def create_path(self, path: str, is_folder: bool, exists_ok: bool = False) -> "File":
        """
        Create a file or folder at the given path, automatically creating parent folders.
        """
        file_parts = path.split("/")
        # create parent folders
        parent = None
        for folder in file_parts[:-1]:
            parent, _ = File.objects.get_or_create(
                project_version=self, parent=parent, name=folder, is_folder=True
            )
        # create file
        file, created = File.objects.get_or_create(
            project_version=self, parent=parent, name=file_parts[-1], is_folder=is_folder
        )
        if not created and not exists_ok:
            raise ValueError(f"file already exists: {file}")
        return file

    def create_file_from_path(self, path: str, exists_ok: bool = False) -> "File":
        return self.create_path(path, is_folder=False, exists_ok=exists_ok)

    def create_folder_from_path(self, path: str, exists_ok: bool = False) -> "File":
        return self.create_path(path, is_folder=True, exists_ok=exists_ok)

    @transaction.atomic
    def define_symbol(
        self,
        name: str,
        content: SymbolContent,
        file: File,
        parent: Optional[SymbolDefinition] = None,
        index: Optional[int] = None,
    ) -> SymbolDefinition:
        """
        Define a symbol in this project version.
        A corresponding symbol is declared if it's not passed.
        """
        # auto set index if not passed
        if index is None:
            if parent:
                index = parent.children.count()
            else:
                index = file.definitions.count()

        return SymbolDefinition.objects.create_definition(
            content=content,
            project_version=self,
            name=name,
            file=file,
            parent=parent,
            index=index,
        )

    def get_symbol_definitions(
        self, name: str, type: Optional[SymbolType] = None
    ) -> QuerySet[SymbolDefinition]:
        """
        Gets the definitions of a symbol in this project version.
        """
        if type is not None:
            return self.available_definitions().filter(name=name, type=type)
        else:
            return self.available_definitions().filter(name=name)

    def get_symbol_definition(
        self, name: str, type: Optional[SymbolType] = None
    ) -> Optional[SymbolDefinition]:
        """
        Gets the definition of a symbol in this project version.

        Raises SymbolDefinition.MultipleObjectsReturned if there are multiple definitions.
        """
        try:
            return self.get_symbol_definitions(name, type).get()
        except SymbolDefinition.MultipleObjectsReturned as e:
            name_dot_type = f"{name} ({type})" if type else name
            raise SymbolDefinition.MultipleObjectsReturned(
                f"multiple definitions for symbol {name_dot_type} in {self}"
            ) from e
        except SymbolDefinition.DoesNotExist:
            return None

    def symbol_definition(self, name: str, type: Optional[SymbolType] = None) -> SymbolDefinition:
        """
        Gets the definition of a symbol in this project version.
        If the definition doesn't exist, we error.
        """
        definition = self.get_symbol_definition(name, type)
        if definition is None:
            available_symbols_str = self._get_available_symbols_debug_str()
            raise ValueError(
                f"symbol {name}{'.' + type if type else ''} is not defined in {self}:\n{available_symbols_str}"
            )
        else:
            return definition

    def _get_available_symbols_debug_str(self, limit: int = 50) -> str:
        available_symbols_count = self.available_definitions().count()
        available_symbols_strs = (str(d) for d in self.available_definitions()[:limit])
        available_symbols_str = (
            f"({min(limit, available_symbols_count)} of {available_symbols_count}"
            f" available symbols: {', '.join(available_symbols_strs)})"
        )
        return available_symbols_str

    def reset(self):
        # deletes all our references and definitions but not their contents
        self.definitions.all().delete()
        self.files.all().delete()

    @transaction.atomic
    def commit(self, name: Optional[str] = None):
        if self.committed:
            raise ValueError(f"already committed: {self}")

        self.committed_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        self.name = name
        # mark all symbol definitions as committed if they aren't already
        # TODO @Performance: commit symbol content server-side in SQL
        for symbol_def in self.definitions.all().prefetch_related(
            "task", "expectation", "instruction", "model", "dataset", "dataset_view"
        ):
            if not symbol_def.committed:
                symbol_def.committed_in = self
                symbol_def.save()
        self.save()

    @gql.model_property(only=["committed_at"])
    def committed(self) -> bool:
        return self.committed_at is not None

    @property
    def organization(self):
        return self.project.organization

    class Meta:
        ordering = ["-created_at"]


class File(UUIDModel):
    """
    A file defining symbols for a project version, potentially containing other files if it's a folder.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="files"
    )
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    is_folder = models.BooleanField(default=False)
    parent = models.ForeignKey("File", on_delete=models.CASCADE, null=True, related_name="files")

    # files via File (if in a folder)
    # definitions via SymbolDefinition

    def __str__(self):
        if self.parent:
            return f"{self.parent}/{self.name}"
        else:
            return f"{self.project_version}/{self.name}"

    @property
    def path(self) -> str:
        return f"{self.parent.path}/{self.name}" if self.parent else self.name

    @property
    def is_root(self) -> bool:
        return self.parent is None

    def add_definition(self, definition: SymbolDefinition):
        definition.file = self
        if definition.parent is None:
            definition.index = self.definitions.count()
        definition.save()

    def create_definition(
        self,
        name: str,
        content: SymbolContent,
        parent: Optional[SymbolDefinition] = None,
    ) -> SymbolDefinition:
        index = self.definitions.count() if parent is None else parent.children.count()
        definition = SymbolDefinition.objects.create_definition(
            name=name,
            content=content,
            project_version=self.project_version,
            parent=parent,
            file=self,
            index=index,
        )
        return definition

    async def acreate_definition(
        self,
        name: str,
        content: SymbolContent,
        parent: Optional[SymbolDefinition] = None,
    ) -> SymbolDefinition:
        return await sync_to_async(self.create_definition)(name, content, parent)

    class Meta:
        ordering = ["name"]
        constraints = [
            # ensure that the path is unique per project version (includes parent folder)
            models.UniqueConstraint(
                name="bench_project_file_project_name_ak",
                fields=["project_version_id", "name"],
                condition=models.Q(parent_id__isnull=True),
            ),
            models.UniqueConstraint(
                name="bench_project_file_project_parent_name_ak",
                fields=["project_version_id", "parent_id", "name"],
                condition=models.Q(parent_id__isnull=False),
            ),
        ]
