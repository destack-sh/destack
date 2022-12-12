from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto

from bench import models
from bench.api import types


@gql.django.input(models.File)
class FileCreateInput:
    project_version: auto
    name: auto
    is_folder: auto
    parent: auto


@gql.django.partial(models.File)
class FileRenameInput(gql.NodeInput):
    name: auto


@gql.django.partial(models.File)
class FileDeleteInput(gql.NodeInput):
    pass


@gql.type
class FileMutation:
    create_file: types.File = gql.django.create_mutation(FileCreateInput)
    rename_file: types.File = gql.django.update_mutation(FileRenameInput)
    delete_file: types.File = gql.django.delete_mutation(FileDeleteInput)
