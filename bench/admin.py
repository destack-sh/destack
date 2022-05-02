from django.contrib import admin

from bench.models import (
    Artifact,
    ArtifactVersion,
    Dataset,
    DatasetVersion,
    Model,
    ModelVersion,
)

admin.site.register(Artifact)
admin.site.register(ArtifactVersion)
admin.site.register(Model)
admin.site.register(ModelVersion)
admin.site.register(Dataset)
admin.site.register(DatasetVersion)
