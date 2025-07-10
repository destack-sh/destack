from collections.abc import Sequence

import structlog
from opentelemetry import trace

from destack.language import Event, NodeType, to_value
from destack.language.registry import NODE_CLASS_BY_TYPE

from ..core import MemoryContext
from .wiring import pack_event_row

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


@tracer.start_as_current_span("memory.execute_append")
def execute_append(context: MemoryContext, events: Sequence[Event]) -> Sequence[Event]:
    """Append events to the hierarchical event tables."""
    for event in events:
        node_type = NodeType(event.metatype)
        event_value = to_value(event, node_as_value=True)
        assert event_value.value is not None, f"no value for {event!r}"
        event_row = pack_event_row(event_value)

        # append to all parent types in hierarchy (including self and event)
        table = context.get_event_table(event_row.ptr)
        current_type = node_type
        while current_type >= NodeType.EVENT:
            table = context.get_event_table(event_row.ptr)

            # add to table
            table.rows[event_row.id] = event_row

            # insert into sorted list
            # binary search for insertion point
            left, right = 0, len(table.rows_sorted)
            while left < right:
                mid = (left + right) // 2
                if table.rows_sorted[mid].created_at <= event_row.created_at:
                    left = mid + 1
                else:
                    right = mid
            table.rows_sorted.insert(left, event_row)

            # move up the hierarchy
            node_cls = NODE_CLASS_BY_TYPE[current_type]
            if node_cls.__base_type__ is not None:
                current_type = node_cls.__base_type__
            else:
                break

    logger.trace("memory.execute_append", events=len(events), span="current")
    return events
