import json

from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.models import (
    Artifact,
    Flow,
    FlowArtifactEdge,
    FlowInstruction,
    FlowInstructionEdge,
    Organization,
)
from bench.models.versioning import get_head


class Command(BaseCommand):
    help = "Updates flow from local JSON files"

    def add_arguments(self, parser: CommandParser):
        parser.add_argument("name", type=str)
        parser.add_argument("--path", type=str, required=True)
        parser.add_argument("--organization", type=str, default="local")

    @transaction.atomic
    def handle(self, *args, **options):
        organization = Organization.objects.get(slug=options["organization"])
        flow, created = Flow.objects.get_or_create(organization=organization, name=options["name"])
        if created:
            flow_version = Flow.objects.create_flow_version_by_name(
                name=options["name"], organization=organization
            )
        else:
            previous_version = get_head(flow)
            flow_version = Flow.objects.create_flow_version_by_name(
                name=options["name"], organization=organization
            )
            flow_version.parents.set([previous_version])

        # read json array from path
        with open(options["path"], "r") as f:
            flow_blocks = json.load(f)

        flow_instructions: list[FlowInstruction] = []
        for i, block in enumerate(flow_blocks):
            block = {**block}  # do not modify original

            try:
                name = block.pop("name")
                function_id = block.pop("function")
                config_arguments = {}

                # TODO @Cleanup: use function spec to parse out config arguments
                if function_id == "bench.text.templatize":
                    config_arguments["template"] = block.pop("template")
                elif function_id == "bench.text.fewshot":
                    config_arguments["template"] = block.pop("template")
                    config_arguments["sample_template"] = block.pop("sample_template")

                flow_instruction = FlowInstruction.objects.create(
                    name=name,
                    function_id=function_id,
                    config_arguments=config_arguments,
                    flow=flow_version,
                )

                # assume remaining arguments are artifact edges
                for argument_name, artifact_name in block.items():
                    artifact = Artifact.objects.get(name=artifact_name, organization=organization)
                    flow_instruction.connected_artifacts.add(
                        get_head(artifact),
                        through_defaults=dict(
                            flow=flow_version,
                            connection_type=FlowArtifactEdge.ConnectionType.Argument,
                            connection_name=argument_name,
                        ),
                    )
                    self.stdout.write(
                        self.style.SUCCESS(f"block {i}: {argument_name} = artifact {artifact_name}")
                    )
            except Exception as e:
                self.stdout.write(self.style.ERROR(f"Error parsing block {i}: {block}"))
                raise e

            # link node to previous node if exists
            if flow_instructions:
                previous_node = flow_instructions[-1]
                flow_instruction.depends_on_nodes.add(
                    previous_node,
                    through_defaults=dict(
                        flow=flow_version,
                        connection_type=FlowInstructionEdge.ConnectionType.Input,
                    ),
                )
            flow_instructions.append(flow_instruction)

        self.stdout.write(self.style.SUCCESS(f"Updated flow with new version: {flow_version}"))
