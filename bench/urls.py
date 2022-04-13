"""
bench URL Configuration
The `urlpatterns` list routes URLs to views. For more information please see:
    https://docs.djangoproject.com/en/4.0/topics/http/urls/
"""
from collections import Callable
from typing import Optional

from django.contrib import admin
from django.urls import URLPattern, path, re_path


def path_with_opt_slash(
    route: str, view: Callable, name: Optional[str] = None
) -> URLPattern:
    """Catches path with or without trailing slash, taking into account query param and hash."""
    # Ignoring the type because while name can be optional on re_path, mypy doesn't agree
    return re_path(rf"^{route}/?(?:[?#].*)?$", view, name=name)  # type: ignore


urlpatterns = [
    path("admin/", admin.site.urls),
]
