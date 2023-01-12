from django.contrib import admin

from bench.models import Organization, Project, ProjectVersion, User

admin.site.register(Organization)
admin.site.register(User)
admin.site.register(Project)
admin.site.register(ProjectVersion)
