from django.db import models
from django.db.models import Q

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel

TAG_KIND_HEAD = "head"
TAG_KIND_BRANCH = "branch"
TAG_KIND_STAGE = "stage"
TAG_KIND_ALIAS = "alias"
TAG_KIND_CAPABILITY = "capability"


class Tag(UUIDModel):
    """
    A generic label for associating groups and/or parts of primitives like
    artifacts, functions and executions with some metadata.

    The tag name can specify a kind by prefixing '<kind>:' to its name. Certain kinds of tags
    have special rules on when & how they can be used and on what metadata they need.
    """

    name = models.SlugField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    metadata = models.JSONField(default=dict)

    class Meta:
        constraints = [models.UniqueConstraint(name="bench_tag_name", fields=["name"])]


RELATED_MODELS = ("artifact", "artifact_version", "artifact_view", "flow", "flow_version")


def make_single_field_populated_check():
    any_single_populated_qs: list[Q] = []
    for field in RELATED_MODELS:
        single_populated_qs = [
            (f"{other_field}__isnull", other_field != field) for other_field in RELATED_MODELS
        ]
        any_single_populated_qs.append(Q(*single_populated_qs, _connector="AND"))
    return models.CheckConstraint(
        name="bench_taggeditem_single_set", check=Q(*any_single_populated_qs, _connector="OR")
    )


def make_uniqueness_checks():
    # Postgres requires partial uniqueness checks for each non-null subset of fields
    def make_partial_uniqueness_check(field: str):
        return models.UniqueConstraint(
            fields=["tag", field],
            name=f"bench_taggeditem_{field}_ak",
            condition=Q((f"{field}__isnull", False)),
        )

    return (make_partial_uniqueness_check(field) for field in RELATED_MODELS)


class TaggedItem(UUIDModel):
    """
    Tagged items track Tag<->Model relationships in a single place.
    """

    tag = models.ForeignKey(Tag, on_delete=models.CASCADE, related_name="tagged_items")

    artifact = models.ForeignKey(
        "Artifact", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )
    artifact_version = models.ForeignKey(
        "ArtifactVersion", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )
    artifact_view = models.ForeignKey(
        "ArtifactView", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )
    flow = models.ForeignKey(
        "Flow", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )
    flow_version = models.ForeignKey(
        "FlowVersion", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )

    class Meta:
        # enforce only one related model field is set and tag + model field are unique
        constraints = [make_single_field_populated_check(), *make_uniqueness_checks()]
