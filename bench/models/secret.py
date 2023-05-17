from django.db import models
from pgcrypto import fields

from bench.models.utils import UUIDModel


class Secret(UUIDModel):
    """
    An encrypted secret.
    """

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="secrets")
    sha512 = models.CharField(max_length=128)
    name = models.CharField(max_length=255, null=True, blank=True)
    value = fields.TextPGPSymmetricKeyField()

    def __str__(self):
        return f"{self.id} ({self.sha512})"

    def __repr__(self):
        return f"<Secret {self}>"
