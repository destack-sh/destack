"""
bench URL Configuration
The `urlpatterns` list routes URLs to views. For more information please see:
    https://docs.djangoproject.com/en/4.0/topics/http/urls/
"""
from django.conf.urls import include
from django.contrib import admin
from django.urls import path
from drf_spectacular.views import SpectacularAPIView, SpectacularRedocView, SpectacularSwaggerView

from bench.api import list_dataset_handlers, list_function_handlers, list_model_handlers
from bench.api import routers as api_routers
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
    *[path(API_PREFIX, include(r.urls)) for r in api_routers],
    path(API_PREFIX + "meta/models", list_model_handlers),
    path(API_PREFIX + "meta/datasets", list_dataset_handlers),
    path(API_PREFIX + "meta/functions", list_function_handlers),
]
