from django.core.validators import validate_slug
from django.db import models

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class Project(TaggableMixin, UUIDModel):
    """
    A project to instruct an AI to do something.

    A project has a main program (the top-level task & flow implementations).
    Later, projects may also be "non-executable" libraries.
    """

    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    slug: models.SlugField = models.SlugField(
        max_length=128, unique=True, validators=[validate_slug]
    )
    description: models.CharField = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="projects"
    )
    # we'll likely have multiple programs per project at some point
    program: models.ForeignKey = models.ForeignKey(
        "Flow", on_delete=models.CASCADE, related_name="projects"
    )
    # flows via Flow.project
    # models via Model.project
    # datasets via Dataset.project
