from collections import defaultdict
from uuid import UUID

from django.core.management import BaseCommand
from django.core.management.base import CommandParser

from bench.executor import executor
from bench.executor.base import FlowExecutionOptions
from bench.function.spec import update_flow_spec
from bench.models import Flow, FlowVersion, Organization
from bench.models.versioning import get_head
from bench.utils.record import RecordList


class Command(BaseCommand):
    help = "Runs a flow with the given inputs & arguments"

    def add_arguments(self, parser: CommandParser):
        parser.add_argument("name", type=str)
        parser.add_argument("--organization", type=str, default="local")
        # arbitrary inputs like --input [node_name:]argument_name=value
        parser.add_argument("--input", type=str, action="append", default=[])
        parser.add_argument("--argument", type=str, action="append", default=[])

    def handle(self, *args, **options):
        organization = Organization.objects.get(slug=options["organization"])
        flow = Flow.objects.get(organization=organization, name=options["name"])
        flow_version: FlowVersion = get_head(flow)
        first_node = flow_version.first_node

        # TODO @Cleanup: remove hack to update spec (shouldn't be needed soon)
        updated_nodes = update_flow_spec(flow_version)
        for node in updated_nodes:
            node.save()

        inputs: dict[UUID, dict[str, RecordList]] = defaultdict(dict)
        for input in options["input"]:
            # parse [node_name:]argument_name=value
            key, value = input.split("=")
            if ":" in key:
                node_name, argument_name = key.split(":")
            else:
                # if node name is not specified, use first node (assumes flow is linear)
                node_name = first_node.name
                argument_name = key
            # convert node name to node id because the executor is annoying right now
            node_id = flow_version.get_node_by_name(node_name).id
            connection_name = "*"

            # stack inputs for each node (assumes record inputs)
            if argument_name in inputs[node_id]:
                inputs[node_id][connection_name]._records.append({argument_name: value})
            else:
                inputs[node_id][connection_name] = RecordList([{argument_name: value}])
        arguments = {}

        inputs_str = ", ".join(f"{node_name}={inputs}" for node_name, inputs in inputs.items())
        self.stdout.write(
            self.style.SUCCESS(f"Running flow {flow_version} with inputs {inputs_str or '<none>'}")
        )
        execution_options = FlowExecutionOptions.default_blocking()
        execution, _ = executor.run_flow(flow_version, inputs, arguments, execution_options)
        self.stdout.write(self.style.SUCCESS(f"Completed flow {flow_version}"))
