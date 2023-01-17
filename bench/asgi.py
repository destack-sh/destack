"""
ASGI config for bench project.

It exposes the ASGI callable as a module-level variable named ``application``.

For more information on this file, see
https://docs.djangoproject.com/en/4.1/howto/deployment/asgi/
and
https://channels.readthedocs.io/en/latest/deploying.html
"""

import os

from channels.auth import AuthMiddlewareStack
from channels.routing import ProtocolTypeRouter, URLRouter
from channels.security.websocket import AllowedHostsOriginValidator
from django.core.asgi import get_asgi_application
from django.urls import re_path
from starlette.middleware.cors import CORSMiddleware
from strawberry.channels import GraphQLHTTPConsumer, GraphQLWSConsumer

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "bench.settings")
django_asgi_app = get_asgi_application()

# import Strawberry schema after creating the django ASGI application
# (ensures django.setup() has been called before any ORM models are imported)
from bench.api import schema  # noqa

websocket_urlpatterns = [
    re_path(r"graphql", GraphQLWSConsumer.as_asgi(schema=schema)),
]

# TODO @Performance @Security: standardize and move CORS handling into nginx?
gql_http_consumer = CORSMiddleware(
    AuthMiddlewareStack(GraphQLHTTPConsumer.as_asgi(schema=schema)),
    allow_origins=["*"],
    allow_methods=["*"],
)
gql_ws_consumer = GraphQLWSConsumer.as_asgi(schema=schema)
application = ProtocolTypeRouter(
    {
        "http": URLRouter([re_path("^graphql", gql_http_consumer), re_path("^", django_asgi_app)]),
        "websocket": AllowedHostsOriginValidator(
            AuthMiddlewareStack(URLRouter(websocket_urlpatterns))
        ),
    }
)
