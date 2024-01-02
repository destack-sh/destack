from django.db import models
from pgcrypto import fields

from bench.models.utils import CrudNode


class Secret(CrudNode):
    """
    An encrypted secret.
    """

    bench = models.ForeignKey("Bench", on_delete=models.CASCADE, related_name="secrets")
    sha512 = models.CharField(max_length=128)
    name = models.CharField(max_length=255, null=True, blank=True)
    value = fields.TextPGPSymmetricKeyField()

    @property
    def parent(self):
        return None

    @property
    def parent_id(self):
        return None

    def __str__(self):
        return f"{self.id} ({self.sha512})"

    def __repr__(self):
        return f"<Secret {self}>"

    class Meta:
        managed = False
