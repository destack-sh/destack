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

from channels.auth import AuthMiddlewareStack
from channels.routing import ProtocolTypeRouter, URLRouter
from channels.security.websocket import AllowedHostsOriginValidator
from django.core.asgi import get_asgi_application
from django.urls import re_path
from starlette.middleware.cors import CORSMiddleware
from strawberry.channels import GraphQLHTTPConsumer, GraphQLWSConsumer
from twisted.internet import reactor

from bench.settings import (
    RUN_INTERNAL_SERVER,
    RUN_RUNTIME_WORKER,
    ZMQ_API_SERVER_ADDR,
    ZMQ_INTERNAL_SERVER_ADDR,
    ZMQ_RUNTIME_WORKER_ADDR,
)
from bench.utils.func import wrap_task

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

# TODO @Cleanup: move internal server & worker startup to proper daphne startup hook
#  For now I just couldn't find the appropriate place to run this, so we rely
#  on the fact that Daphne uses reactor's _asyncioEventLoop to create tasks there.
if RUN_INTERNAL_SERVER:
    from bench.runtime.dbserver import InternalServer

    server = InternalServer()
    coro = server.start(
        internal_server_addr=ZMQ_INTERNAL_SERVER_ADDR, api_server_addr=ZMQ_API_SERVER_ADDR
    )
    task = reactor._asyncioEventloop.create_task(wrap_task(coro, "internal_server"))
    reactor.addSystemEventTrigger("before", "shutdown", server.stop)

if RUN_RUNTIME_WORKER:
    from bench.runtime.worker import RuntimeWorker

    local_id = random.randint(0, 2 ** 32)  # just some random number
    worker = RuntimeWorker(worker_id=f"local.{hex(local_id)[2:]}")
    coro = worker.start(
        runtime_worker_addr=ZMQ_RUNTIME_WORKER_ADDR,
        internal_server_addr=ZMQ_INTERNAL_SERVER_ADDR,
        api_server_addr=ZMQ_API_SERVER_ADDR,
    )
    task = reactor._asyncioEventloop.create_task(wrap_task(coro, "runtime_worker"))
    reactor.addSystemEventTrigger("before", "shutdown", worker.stop)
