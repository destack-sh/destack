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

from bench.utils.cache import test_redis_connection

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "bench.settings")
django_asgi_app = get_asgi_application()

reactor._asyncioEventloop.create_task(test_redis_connection())
