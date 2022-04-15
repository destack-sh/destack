from django.db import models

from bench.models.utils import MAX_NAME_LENGTH, UUIDModel


class Model(UUIDModel):
    name = models.CharField(max_length=MAX_NAME_LENGTH)


class HuggingFaceModel(Model):
    pass


class SpacyModel(Model):
    pass
