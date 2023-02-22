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
    CORS_ALLOWED_ORIGINS,
    DEBUG,
    RUN_INTSERVER,
    RUN_WORKER,
    TEST,
    ZMQ_API_PUB_ADDR,
    ZMQ_INTSERVER_PUB_ADDR,
    ZMQ_INTSERVER_REP_ADDR,
    ZMQ_WORKER_PUB_ADDR,
    ZMQ_WORKER_REP_ADDR,
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

gql_http_consumer = CORSMiddleware(
    AuthMiddlewareStack(GraphQLHTTPConsumer.as_asgi(schema=schema)),
    allow_origins=CORS_ALLOWED_ORIGINS,
    allow_methods=["*"],
    allow_credentials=True,
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
if RUN_INTSERVER:
    from bench.runtime.intserver import InternalServer

    # Bind internal server's api socket to localhost if it's a wildcard,
    # because wildcard means we're also hosting the API server, but ZMQ obviously
    # can't connect to wildcard. Likewise, we do the same for intserver in API.
    ZMQ_API_PUB_ADDR = ZMQ_API_PUB_ADDR.replace("*", "127.0.0.1")

    server = InternalServer()
    coro = server.run(
        intserver_rep_addr=ZMQ_INTSERVER_REP_ADDR,
        intserver_pub_addr=ZMQ_INTSERVER_PUB_ADDR,
        api_pub_addr=ZMQ_API_PUB_ADDR,
        worker_pub_addr=ZMQ_WORKER_PUB_ADDR,
    )
    task = reactor._asyncioEventloop.create_task(wrap_task(coro, "intserver"))
    reactor.addSystemEventTrigger("before", "shutdown", server.stop)
if RUN_WORKER:
    if not DEBUG or TEST:
        raise RuntimeError("worker should be run via isolated runworker in prod")
    from bench.runtime.worker import RuntimeWorker

    local_id = random.randint(0, 2 ** 32)  # just some random number
    worker = RuntimeWorker(worker_id=f"local.{hex(local_id)[2:]}")
    coro = worker.run(
        worker_rep_addr=ZMQ_WORKER_REP_ADDR,
        worker_pub_addr=ZMQ_WORKER_PUB_ADDR,
        intserver_rep_addr=ZMQ_INTSERVER_REP_ADDR,
        intserver_pub_addr=ZMQ_INTSERVER_PUB_ADDR,
    )
    task = reactor._asyncioEventloop.create_task(wrap_task(coro, "worker"))
    reactor.addSystemEventTrigger("before", "shutdown", worker.stop)
