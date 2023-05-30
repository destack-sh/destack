from django.db import models

from bench.models.utils import UUIDModel
from bench.utils.uuidt import MAX_NAME_LENGTH


class Tile(UUIDModel):
    """
    An element on a screen.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="tiles"
    )
    screen = models.ForeignKey("Screen", on_delete=models.CASCADE, related_name="tiles")
    revision = models.IntegerField(default=0)
    name = models.CharField(max_length=MAX_NAME_LENGTH, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    deleted_at = models.DateTimeField(null=True, blank=True)
    parent = models.ForeignKey(
        "Tile", on_delete=models.CASCADE, related_name="children", null=True, blank=True
    )
    order_key = models.CharField(max_length=64)  # in screen/parent
    x = models.IntegerField(default=0)
    y = models.IntegerField(default=0)

    children: models.QuerySet[Tile]  # noqa via Tile.parent


class Screen(UUIDModel):
    """
    A custom UI made of tiles.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="screens"
    )
    name = models.CharField(max_length=MAX_NAME_LENGTH, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    deleted_at = models.DateTimeField(null=True, blank=True)
    revision = models.IntegerField(default=0)

    tiles: models.QuerySet[Tile]  # noqa via Tile.screen
