"""
bench URL Configuration
The `urlpatterns` list routes URLs to views. For more information please see:
    https://docs.djangobench.com/en/4.0/topics/http/urls/
"""
from django.contrib import admin
from django.http import HttpResponse
from django.urls import path
from django.views.decorators.csrf import csrf_exempt


@csrf_exempt
def empty_view(request):
    return HttpResponse(status=204)


urlpatterns = [
    # empty index
    path("", empty_view, name="index"),
    path("admin/", admin.site.urls),
]
