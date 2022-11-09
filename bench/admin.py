from django.contrib import admin

from bench.models import (
    Dataset,
    Execution,
    Flow,
    Model,
    Organization,
    Project,
    ProjectVersion,
    Task,
    User,
)

admin.site.register(Organization)
admin.site.register(User)
admin.site.register(Project)
admin.site.register(ProjectVersion)
admin.site.register(Task)
admin.site.register(Flow)
admin.site.register(Model)
admin.site.register(Dataset)
admin.site.register(Execution)
