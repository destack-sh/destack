"""
bench URL Configuration
The `urlpatterns` list routes URLs to views. For more information please see:
    https://docs.djangoproject.com/en/4.0/topics/http/urls/
"""
from django.conf.urls import include
from django.contrib import admin
from django.http import HttpResponse
from django.urls import path
from django.views.decorators.csrf import csrf_exempt
from drf_spectacular.views import SpectacularAPIView
from strawberry.django.views import GraphQLView

from bench.api.rest import run
from bench.api.root import schema
from bench.settings import DEBUG


@csrf_exempt
def empty_view(request):
    return HttpResponse(status=204)


urlpatterns = [
    # empty index
    path("", empty_view, name="index"),
    path("admin/", admin.site.urls),
    path("", include("django_prometheus.urls")),
    path("", include("social_django.urls", namespace="social")),
    path("schema", SpectacularAPIView.as_view(), name="schema"),
    path("<owner>/<project>/run", run, name="run"),
    path(
        "graphql",
        GraphQLView.as_view(schema=schema, graphiql=DEBUG, allow_queries_via_get=False),
        name="graphql",
    ),
]
