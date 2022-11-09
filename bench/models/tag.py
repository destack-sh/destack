from typing import TYPE_CHECKING, Optional, Union

import cachetools
from django.db import models
from django.db.models import Q

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel

if TYPE_CHECKING:
    from bench.models import Organization
else:
    Organization = object


class TagManager(models.Manager):
    def of_type(self, typ: str) -> models.QuerySet:
        return Tag.objects.filter(name__startswith=typ + ":")


class Tag(UUIDModel):
    """
    A generic label for associating groups and/or parts of primitives like
    artifacts, functions and executions with some metadata.

    The tag name can specify a type by prefixing '<type>:' to its name. Certain kinds of tags
    have special rules on when & how they can be used and on what metadata they need.
    """

    name = models.SlugField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    metadata = models.JSONField(default=dict, null=True)

    organization: models.ForeignKey = models.ForeignKey(
        "bench.Organization", on_delete=models.CASCADE, related_name="tags"
    )

    objects = TagManager()

    @property
    def type(self) -> Optional[str]:
        name_parts = self.name.split(":")
        if len(name_parts) != 1 and len(name_parts[0]) >= 1:
            return name_parts[0]
        else:
            return None

    class Meta:
        constraints = [models.UniqueConstraint(name="bench_tag_name", fields=["name"])]


# Must keep in sync with the actual fields of TaggedItem.
RELATED_FIELDS = (
    "model",
    "dataset",
    "dataset_version",
    "dataset_view",
    "flow",
    "flow_version",
    "project",
    "project_version",
    "task",
)
RELATED_MODELS = (
    "Model",
    "Dataset",
    "DatasetVersion",
    "DatasetView",
    "Flow",
    "FlowVersion",
    "Project",
    "ProjectVersion",
    "Task",
)


def make_single_field_populated_check():
    any_single_populated_qs: list[Q] = []
    for field in RELATED_FIELDS:
        single_populated_qs = [
            (f"{other_field}__isnull", other_field != field) for other_field in RELATED_FIELDS
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

    return (make_partial_uniqueness_check(field) for field in RELATED_FIELDS)


class TaggedItem(UUIDModel):
    """
    Tagged items track Tag<->Model relationships in a single place.
    """

    tag = models.ForeignKey(Tag, on_delete=models.CASCADE, related_name="tagged_items")

    model = models.ForeignKey(
        "Model", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )
    dataset = models.ForeignKey(
        "Dataset", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )
    dataset_version = models.ForeignKey(
        "DatasetVersion", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )
    dataset_view = models.ForeignKey(
        "DatasetView", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )
    flow = models.ForeignKey(
        "Flow", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )
    flow_version = models.ForeignKey(
        "FlowVersion", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )
    project = models.ForeignKey(
        "Project", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )
    task = models.ForeignKey(
        "Task", on_delete=models.CASCADE, related_name="tagged_items", null=True
    )

    class Meta:
        # enforce only one related model field is set and tag + model field are unique
        constraints = [make_single_field_populated_check(), *make_uniqueness_checks()]


class TaggableMixin:
    """Tags utilities for models that can be tagged (via TaggedItem)"""

    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)

        # Help anyone thinking that adding 'TaggableMixin' is enough to make a model taggable.
        if (
            hasattr(cls, "_meta")
            and cls._meta.concrete_model.__name__ not in RELATED_MODELS
            and cls.__name__ not in RELATED_MODELS
            and cls.__module__ != "__fake__"  # used during SQLite migrations
        ):
            raise RuntimeError(f"Taggable models must have fields in TaggedItem: {cls}")

    tagged_items: models.QuerySet

    def set_tags(self, tags: list[str]):
        current_tags = set(tags)  # deduplicate
        # create/set new tags
        tagged_item_instances = []
        for tag in current_tags:
            tag_instance, _ = Tag.objects.get_or_create(name=tag)
            tagged_item_instance, _ = self.tagged_items.get_or_create(tag_id=tag_instance.id)
            tagged_item_instances.append(tagged_item_instance)
        # delete extraneous tagged items
        self.tagged_items.exclude(tag__name__in=current_tags).delete()

        self.prefetched_tags = tagged_item_instances

    def set_tag(self, tag: Union[Tag, str]) -> Tag:
        if isinstance(tag, str):
            tag_instance, _ = Tag.objects.get_or_create(name=tag)
        else:
            tag_instance = tag
        self.tagged_items.get_or_create(tag_id=tag_instance.id)
        return tag_instance


TAG_TYPE_HEAD = "head"
TAG_TYPE_BRANCH = "branch"
TAG_TYPE_RELEASE = "release"
TAG_TYPE_STAGE = "stage"
TAG_TYPE_ALIAS = "alias"
TAG_TYPE_CAPABILITY = "capability"
TAG_TYPE_SOURCE = "source"


def _make_tag(
    name: str, organization: Organization, description: str, metadata: Optional[dict]
) -> Tag:
    tag, _ = Tag.objects.get_or_create(
        organization=organization,
        name=name,
        defaults=dict(description=description, metadata=metadata),
    )
    return tag


_DEFAULT_TAG_PROPERTIES = {
    "source:inputs": ("Artifacts representing inputs", None),
    "source:outputs": ("Artifacts representing inputs", None),
}


@cachetools.cached(cache={})
def default_tag(name: str, organization: Organization) -> Tag:
    if name not in _DEFAULT_TAG_PROPERTIES:
        raise ValueError(f"unknown default tag: {name}")

    description, metadata = _DEFAULT_TAG_PROPERTIES[name]
    return _make_tag(name, organization, description, metadata)
