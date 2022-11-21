"""
bench URL Configuration
The `urlpatterns` list routes URLs to views. For more information please see:
    https://docs.djangoproject.com/en/4.0/topics/http/urls/
"""
from django.conf.urls import include
from django.contrib import admin
from django.urls import path
from drf_spectacular.views import SpectacularAPIView
from strawberry.django.views import GraphQLView

from bench.api import schema
from bench.api.rest import run_program
from bench.settings import DEBUG

urlpatterns = [
    path("admin/", admin.site.urls),
    path("", include("django_prometheus.urls")),
    path("", include("social_django.urls", namespace="social")),
    path("schema", SpectacularAPIView.as_view(), name="schema"),
    path("<organization>/<project>/run", run_program, name="run"),
    path(
        "graphql",
        GraphQLView.as_view(schema=schema, graphiql=DEBUG, allow_queries_via_get=False),
        name="graphql",
    ),
]
