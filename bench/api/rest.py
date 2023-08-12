import threading
from functools import wraps
from typing import NamedTuple, Optional
from uuid import UUID

import structlog
from django.core.exceptions import ObjectDoesNotExist, PermissionDenied, ValidationError
from django.db.models import Q
from django.http import HttpRequest, HttpResponse, JsonResponse
from rest_framework import serializers

from bench.models import Project, ProjectVersion
from bench.models.token import AccessTokenScope
from bench.utils.dt import utcnow_with_tz

logger = structlog.get_logger(__name__)


class RunInputSerializer(serializers.Serializer):
    version = serializers.CharField(default="x")  # project version tag
    task = serializers.CharField(default=None, allow_null=True)
    code = serializers.CharField(default=None, allow_null=True)
    build = serializers.CharField(default=None, allow_null=True)
    inputs = serializers.JSONField(default=None, allow_null=True)
    block = serializers.BooleanField(default=True)


class RunOutputSerializer(serializers.Serializer):
    run_id = serializers.UUIDField()
    outputs = serializers.JSONField(allow_null=True)
    success = serializers.BooleanField()
    error = serializers.JSONField(allow_null=True)


def async_csrf_exempt(view_func):
    async def wrapped_view(request, *args, **kwargs):
        return await view_func(request, *args, **kwargs)

    wrapped_view.csrf_exempt = True
    return wraps(view_func)(wrapped_view)


def async_check_is_main_thread(view_func):
    async def wrapped_view(request, *args, **kwargs):
        if threading.current_thread().name != "django-main-thread":
            # someone fucked up
            logger.error(
                "calling_from_non_main",
                view_func=view_func,
                current_thread=threading.current_thread(),
                main_thread=threading.main_thread(),
            )
            return HttpResponse(status=500)
        return await view_func(request, *args, **kwargs)

    return wraps(view_func)(wrapped_view)


def async_api_view(methods: list[str] = None):
    """DRF's api view does not support async, so we make our own."""

    def make_view(view_func):
        async def wrapped_view(request, *args, **kwargs):
            if methods is not None and request.method not in methods:
                return HttpResponse(status=405)
            try:
                return await view_func(request, *args, **kwargs)
            except (serializers.ValidationError, ValidationError) as e:
                return JsonResponse(e.detail, safe=False, status=400)
            except PermissionDenied:
                return HttpResponse(status=403)
            except ObjectDoesNotExist:
                return HttpResponse(status=404)

        wrapped_view.methods = methods
        return wraps(view_func)(wrapped_view)

    return make_view


AccessInfo = NamedTuple(
    "AccessInfo",
    [
        ("project_version_id", UUID),
        ("access_token_id", UUID),
        ("organization_id", Optional[UUID]),
        ("user_id", Optional[UUID]),
    ],
)


def get_access(
    owner: str, project: str, tag: str, token_digest: str, scope=AccessTokenScope.RUN
) -> AccessInfo:
    # TODO @Feature: implement semver range tags? https://devhints.io/semver
    # TODO @Performance: cache get_access & fetch in one SQL query (incl. access token check)
    if tag in ("*", "^", "x"):
        # use latest version
        project_version = Project.objects.get_by_slug(owner, project).head
    else:
        project_version = ProjectVersion.objects.get_by_slug(owner, project, tag=tag)
    access_token = (
        project_version.project.owner.access_tokens.filter(
            digest=token_digest,
            scopes__contains=[scope],
        )
        .filter(Q(revoked_at__isnull=True))
        .filter(Q(expires_at__gte=utcnow_with_tz()) | Q(expires_at__isnull=True))
        .only("id", "user_id", "organization_id")
        .first()
    )
    if access_token is None:
        raise PermissionDenied("cannot access this deployment")
    return AccessInfo(
        project_version.id,
        access_token.id,
        access_token.organization_id,
        access_token.user_id,
    )


@async_csrf_exempt
@async_check_is_main_thread
@async_api_view(methods=["POST"])
async def run(req: HttpRequest, owner: str, project: str) -> HttpResponse:
    raise NotImplementedError
