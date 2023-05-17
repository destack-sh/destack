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
    value = fields.TextPGPPublicKeyField()
