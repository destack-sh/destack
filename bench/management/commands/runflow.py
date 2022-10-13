import json
from collections import defaultdict
from uuid import UUID

from django.core.management import BaseCommand
from django.core.management.base import CommandParser

from bench.dataset.accessor import DatasetAccessor
from bench.executor import executor
from bench.executor.base import FlowExecutionOptions
from bench.function.spec import update_flow_spec
from bench.models import Flow, FlowVersion, Organization
from bench.models.execution import DEFAULT_CONNECTION_NAME
from bench.models.versioning import get_head
from bench.utils.record import RecordList


class Command(BaseCommand):
    help = "Runs a flow with the given inputs & arguments"

    def add_arguments(self, parser: CommandParser):
        parser.add_argument("name", type=str)
        parser.add_argument("--organization", type=str, default="local")
        # whether to only print data outputs
        parser.add_argument("--verbose", action="store_true")
        # arbitrary inputs like --input [node_name:]argument_name=value
        parser.add_argument("--input", type=str, action="append", default=[])
        parser.add_argument("--argument", type=str, action="append", default=[])

    def handle(self, *args, **options):
        verbose = options.get("verbose", False)
        organization = Organization.objects.get(slug=options["organization"])
        flow = Flow.objects.get(organization=organization, name=options["name"])
        flow_version: FlowVersion = get_head(flow)

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
                node_name = flow_version.first_node.name
                argument_name = key
            # convert node name to node id because the executor is annoying right now
            node_id = flow_version.get_node_by_name(node_name).id
            connection_name = DEFAULT_CONNECTION_NAME

            # set inputs on first record
            if connection_name in inputs[node_id]:
                inputs[node_id][connection_name][0][argument_name] = value
            else:
                inputs[node_id][connection_name] = RecordList([{argument_name: value}])
        arguments = {}

        if verbose:
            inputs_str = ", ".join(f"{node_name}={inputs}" for node_name, inputs in inputs.items())
            self.stdout.write(
                self.style.SUCCESS(
                    f"Running flow {flow_version} with inputs {inputs_str or '<none>'}"
                )
            )

        # execute flow
        execution_options = FlowExecutionOptions.default_blocking()
        execution, plan = executor.run_flow(flow_version, inputs, arguments, execution_options)

        if verbose:
            self.stdout.write(self.style.SUCCESS(f"Completed flow {flow_version}"))

        # print outputs
        output = plan.final_outputs[flow_version.last_node.id][DEFAULT_CONNECTION_NAME]
        output_records = DatasetAccessor(output.artifact).get_records_view(output.view_data)
        output_records_data = [record.data for record in output_records]

        output_data_str = json.dumps(output_records_data, indent=4)
        if verbose:
            self.stdout.write(self.style.SUCCESS(f"Output {output.artifact}"))
        self.stdout.write(output_data_str)
