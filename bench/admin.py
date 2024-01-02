from django.contrib import admin

from bench.models import Bench, BenchVersion, Organization, User

admin.site.register(Organization)
admin.site.register(User)
admin.site.register(Bench)
admin.site.register(BenchVersion)
