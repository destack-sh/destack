"""
ASGI config for bench project.

It exposes the ASGI callable as a module-level variable named ``application``.

For more information on this file, see
https://docs.djangoproject.com/en/4.1/howto/deployment/asgi/
and
https://channels.readthedocs.io/en/latest/deploying.html
"""
import os
import random
from uuid import uuid4

from channels.auth import AuthMiddlewareStack
from channels.routing import ProtocolTypeRouter, URLRouter
from channels.security.websocket import AllowedHostsOriginValidator
from django.core.asgi import get_asgi_application
from django.urls import re_path
from starlette.middleware.cors import CORSMiddleware
from strawberry.channels import GraphQLHTTPConsumer, GraphQLWSConsumer
from twisted.internet import reactor

from bench.msg.core import drain_nats, init_nats
from bench.runtime import run
from bench.settings import CORS_ALLOWED_ORIGINS, DEBUG, RUN_INTSERVER, RUN_WORKER, TEST
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

# TODO @Cleanup: move 'sidecar' server tasks to proper startup hooks
#  For now I just couldn't find the appropriate place to run this, so we rely
#  on the fact that Daphne uses reactor's _asyncioEventLoop to create tasks there.

# NATS must always run.
task = reactor._asyncioEventloop.create_task(wrap_task(init_nats()))
reactor.addSystemEventTrigger("before", "shutdown", drain_nats)

if RUN_INTSERVER:
    from bench.runtime.langserver import LanguageServer

    server = LanguageServer()
    coro = server.run()
    task = reactor._asyncioEventloop.create_task(wrap_task(coro, "langserver"))
    reactor.addSystemEventTrigger("before", "shutdown", server.stop)

# for local development only
if RUN_WORKER:
    if not DEBUG or TEST:
        raise RuntimeError("worker should be run via isolated runworker in prod")
    from bench.runtime.worker import SandboxedWorker

    local_id = random.randint(0, 2**32)  # just some random number
    run.ALLOW_UNTRUSTED_CODE = True
    worker = SandboxedWorker(worker_id=uuid4(), deployment_id=None, project_id=None)
    coro = worker.run()
    task = reactor._asyncioEventloop.create_task(wrap_task(coro, "worker"))
    reactor.addSystemEventTrigger("before", "shutdown", worker.stop)
