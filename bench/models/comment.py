import uuid
from typing import Optional

from django.db import models

from bench.language.const import NodeType
from bench.models.utils import CrudModel, DetachedNode, get_choices


class Comment(CrudModel, DetachedNode):
    """A nested comment on a file or statement."""

    parent_ck = models.UUIDField()
    parent_type = models.CharField(max_length=64, choices=get_choices(NodeType))
    parent_comment = models.ForeignKey("Comment", on_delete=models.CASCADE, related_name="children")
    text = models.TextField()

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.parent_comment_id

    @property
    def parent(self) -> Optional["Comment"]:
        return self.parent_comment
