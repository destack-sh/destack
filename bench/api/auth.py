import structlog
from django.contrib.auth import logout as django_logout
from rest_framework.decorators import api_view
from rest_framework.response import Response
from social_django.strategy import DjangoStrategy

from bench.models import User

logger = structlog.get_logger(__name__)


def social_create_user(strategy: DjangoStrategy, details, backend, user=None, *args, **kwargs):
    if user:
        return {"is_new": False}

    username = details.get("username")
    email = details.get("email")
    email = details["email"][0] if isinstance(details["email"], (list, tuple)) else details["email"]
    full_name = (
        details.get("fullname")
        or f"{details.get('first_name') or ''} {details.get('last_name') or ''}".strip()
        or details.get("username")
    )
    user = User.objects.create_user(username, email, full_name)
    strategy.session_set("backend", backend.name)

    logger.info("social_create_user", user=user)
    return {"is_new": True, "user": user}


# plain DRF functional logout view
@api_view(["POST"])
def logout(request):
    django_logout(request)
    # redirect to next url if provided
    next_url = request.data.get("next")
    if next_url:
        return Response(status=302, headers={"Location": next_url})
    return Response(status=204)
