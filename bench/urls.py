"""
bench URL Configuration
The `urlpatterns` list routes URLs to views. For more information please see:
    https://docs.djangoproject.com/en/4.0/topics/http/urls/
"""
from django.conf.urls import include
from django.contrib import admin
from django.urls import path
from drf_spectacular.views import SpectacularAPIView, SpectacularRedocView

from bench.api import model, router

urlpatterns = [
    path("admin/", admin.site.urls),
    path("api/schema", SpectacularAPIView.as_view(), name="schema"),
    path(
        "api/schema/redoc/",
        SpectacularRedocView.as_view(url_name="schema"),
        name="redoc",
    ),
    # typing complains because model.predict is async and django-stubs doesn't handle that
    path("api/models/predict", model.predict),  # type: ignore
    path("api/", include(router.urls)),
]
