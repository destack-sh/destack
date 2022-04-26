"""
bench URL Configuration
The `urlpatterns` list routes URLs to views. For more information please see:
    https://docs.djangoproject.com/en/4.0/topics/http/urls/
"""

from django.contrib import admin
from django.urls import path
from drf_spectacular.views import SpectacularAPIView

from bench.api import model

urlpatterns = [
    path("admin/", admin.site.urls),
    path("api/model/predict", model.predict),  # type: ignore
    path("api/schema", SpectacularAPIView.as_view(), name="schema"),
]
