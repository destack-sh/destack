"""
bench URL Configuration
The `urlpatterns` list routes URLs to views. For more information please see:
    https://docs.djangobench.com/en/4.0/topics/http/urls/
"""
from django.conf.urls import include
from django.contrib import admin
from django.http import HttpResponse
from django.urls import path
from django.views.decorators.csrf import csrf_exempt
from drf_spectacular.views import SpectacularAPIView


@csrf_exempt
def empty_view(request):
    return HttpResponse(status=204)


urlpatterns = [
    # empty index
    path("", empty_view, name="index"),
    path("admin/", admin.site.urls),
    path("", include("django_prometheus.urls")),
    path("schema", SpectacularAPIView.as_view(), name="schema"),
]
