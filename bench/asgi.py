"""
ASGI config for Bench.

It exposes the ASGI callable as a module-level variable named ``application``.

For more information on this file, see
https://docs.djangobench.com/en/4.1/howto/deployment/asgi/
and
https://channels.readthedocs.io/en/latest/deploying.html
"""
import os

from django.core.asgi import get_asgi_application
from twisted.internet import reactor

from bench.settings import (
    RUN_LANGUAGE_SERVER_IN_API,
    RUN_ORCHESTRATION_SERVER_IN_API,
)
from bench.utils.cache import test_redis_connection
from bench.utils.func import wrap_task

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "bench.settings")
django_asgi_app = get_asgi_application()

reactor._asyncioEventloop.create_task(test_redis_connection())

# run servers alongside API server (for development)
if RUN_LANGUAGE_SERVER_IN_API:
    from bench.server import RuntimeSupervisor

    server = RuntimeSupervisor()
    task = reactor._asyncioEventloop.create_task(wrap_task(server.run(), "runtime"))
    reactor.addSystemEventTrigger("before", "shutdown", server.stop)

if RUN_ORCHESTRATION_SERVER_IN_API:
    from bench.server import ComputeOrchestrator

    server = ComputeOrchestrator()
    task = reactor._asyncioEventloop.create_task(wrap_task(server.run(), "master"))
    reactor.addSystemEventTrigger("before", "shutdown", server.stop)
