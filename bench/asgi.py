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
from twisted.internet import reactor

from bench.msg.core import drain_nats, init_nats, process_soon_queue
from bench.settings import (
    CORS_ALLOWED_ORIGINS,
    RUN_LANGUAGE_SERVER_IN_API,
    RUN_ORCHESTRATION_SERVER_IN_API,
)
from bench.utils.cache import test_redis_connection
from bench.utils.func import wrap_task

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "bench.settings")
django_asgi_app = get_asgi_application()

# import Strawberry schema after creating the django ASGI application
# (ensures django.setup() has been called before any ORM models are imported)
from bench.api import schema  # noqa

websocket_urlpatterns = [
    re_path(r"graphql", GraphQLWSConsumer.as_asgi(schema=schema)),
]

gql_http_consumer = CORSMiddleware(
    AuthMiddlewareStack(GraphQLHTTPConsumer.as_asgi(schema=schema)),
    allow_origins=CORS_ALLOWED_ORIGINS,
    # see https://docs.sentry.io/platforms/javascript/guides/react/performance/instrumentation/automatic-instrumentation
    allow_headers=["sentry-trace", "baggage", "x-client-nonce"],
    allow_methods=["*"],
    allow_credentials=True,
)

gql_ws_consumer = GraphQLWSConsumer.as_asgi(schema=schema)
application = ProtocolTypeRouter(
    {
        "http": (
            URLRouter([re_path("^graphql", gql_http_consumer), re_path("^", django_asgi_app)])
        ),
        "websocket": AllowedHostsOriginValidator(
            AuthMiddlewareStack(URLRouter(websocket_urlpatterns))
        ),
    }
)

# start NATS
task = reactor._asyncioEventloop.create_task(wrap_task(init_nats()))
reactor.addSystemEventTrigger("before", "shutdown", drain_nats)

# start 'soon' publish queue
task = reactor._asyncioEventloop.create_task(wrap_task(process_soon_queue()))
reactor._asyncioEventloop.create_task(test_redis_connection())

# run servers alongside API server (for development)
if RUN_LANGUAGE_SERVER_IN_API:
    from bench.server import LanguageServer

    server = LanguageServer()
    task = reactor._asyncioEventloop.create_task(wrap_task(server.run(), "langserver"))
    reactor.addSystemEventTrigger("before", "shutdown", server.stop)

if RUN_ORCHESTRATION_SERVER_IN_API:
    from bench.server import OrchestrationServer

    server = OrchestrationServer()
    task = reactor._asyncioEventloop.create_task(wrap_task(server.run(), "master"))
    reactor.addSystemEventTrigger("before", "shutdown", server.stop)
