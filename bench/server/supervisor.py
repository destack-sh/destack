import structlog

from bench.proto.wire import GlobalSupervisorBase
from bench.utils.monitoring import Monitored

logger = structlog.get_logger(__name__)

WORKER_SET_IDLE_SLEEP_TIME = 30 * 60  # 30 minutes
WORKER_SET_GENTLE_RESTART_TIMEOUT = 5  # 5 seconds until force restart


class GlobalSupervisor(Monitored, GlobalSupervisorBase):
    pass
