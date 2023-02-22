import structlog
from social_django.strategy import DjangoStrategy

from bench.models import User

logger = structlog.get_logger(__name__)


def signup_create_user(strategy: DjangoStrategy, details, backend, user=None, *args, **kwargs):
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

    return {"is_new": True, "user": user}
