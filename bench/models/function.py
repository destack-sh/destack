from typing import TYPE_CHECKING

from django.db import models
from django.db.models import QuerySet

from bench.models.utils import (
    DATASET_TYPE,
    MAX_DESCRIPTION_LENGTH,
    MAX_NAME_LENGTH,
    MODEL_TYPE,
    UUIDModel,
    proxies,
)

if TYPE_CHECKING:
    from bench.models import DatasetVersion, ModelVersion


class FunctionManager(models.Manager):
    pass


class Function(UUIDModel):
    """
    A function is a curried variant of a registered data function with given arguments,
    including any required "init-time" artifacts like datasets and models.

    A function may be managed by or dependent on an external Controller, which provides
    additional configuration arguments and/or augments the function config & execution.
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    controller = models.ForeignKey(
        "Controller", on_delete=models.RESTRICT, blank=True, null=True
    )

    objects = FunctionManager()


class FunctionVersion(UUIDModel):
    function = models.ForeignKey(
        "Function", related_name="versions", on_delete=models.CASCADE
    )
    version = models.CharField(max_length=256)
    created_at = models.DateTimeField(auto_now_add=True)

    registered_id = models.CharField(max_length=256)
    config_arguments = models.JSONField()
    # TODO @Cleanup: simplify function to function references w.r.t. versioning?
    functions = models.ManyToManyField(
        "FunctionVersion", through="FunctionFunctionArgument"
    )
    artifacts = models.ManyToManyField(
        "ArtifactVersion", through="FunctionArtifactArgument"
    )

    @property
    def models(self) -> QuerySet["ModelVersion"]:
        return proxies(
            self.artifacts.filter(artifact__type__exact=MODEL_TYPE), ModelVersion
        )

    @property
    def datasets(self) -> QuerySet["DatasetVersion"]:
        return proxies(
            self.artifacts.filter(artifact__type__exact=DATASET_TYPE), DatasetVersion
        )

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_function_version_ak", fields=["function", "version"]
            )
        ]


class FunctionFunctionArgument(UUIDModel):
    caller_function = models.ForeignKey(
        "FunctionVersion",
        on_delete=models.RESTRICT,
        related_name="called_functions",
    )
    callee_function = models.ForeignKey(
        "FunctionVersion",
        on_delete=models.RESTRICT,
        related_name="caller_functions",
    )


class FunctionArtifactArgument(UUIDModel):
    function = models.ForeignKey("FunctionVersion", on_delete=models.RESTRICT)
    artifact = models.ForeignKey("ArtifactVersion", on_delete=models.RESTRICT)
