import structlog

from bench.proto.wire import ModuleHostBase
from bench.utils.monitoring import Monitored

logger = structlog.get_logger(__name__)

# :MinTriggerInterval (because less than pre send window won't work)
TIME_TRIGGER_PRE_SEND_WINDOW = 45  # seconds
TIME_TRIGGER_LOOKAHEAD = 2  # occurrences


class ModuleHost(Monitored, ModuleHostBase):
    pass
