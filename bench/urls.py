"""
bench URL Configuration
The `urlpatterns` list routes URLs to views. For more information please see:
    https://docs.djangoproject.com/en/4.0/topics/http/urls/
"""
from django.conf.urls import include
from django.contrib import admin
from django.urls import path
from drf_spectacular.views import SpectacularAPIView, SpectacularRedocView, SpectacularSwaggerView
from strawberry.django.views import GraphQLView

from bench.api import schema
from bench.settings import API_PREFIX

urlpatterns = [
    path("admin/", admin.site.urls),
    path("", include("django_prometheus.urls")),
    path("", include("social_django.urls", namespace="social")),
    path(API_PREFIX + "schema", SpectacularAPIView.as_view(), name="schema"),
    path(
        API_PREFIX + "schema/redoc/", SpectacularRedocView.as_view(url_name="schema"), name="redoc"
    ),
    path(
        API_PREFIX + "schema/swagger/",
        SpectacularSwaggerView.as_view(url_name="schema"),
        name="swagger",
    ),
    path("graphql", GraphQLView.as_view(schema=schema)),
]
