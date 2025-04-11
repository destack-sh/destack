import structlog
from opentelemetry import trace

from bench.language import NodeType, Trigger, bittuple
from bench.system.host import HostPlugin

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# NOTE :Incomplete: consider runtime Triggers (i.e., those not in source)
# NOTE :Architecture: how will Triggers work with hibernated Runs? :HibernateRuns


class ScheduleTriggerPlugin(HostPlugin[Trigger]):
    """Process ScheduleTriggers."""

    watch_types = bittuple(NodeType.TRIGGER)
