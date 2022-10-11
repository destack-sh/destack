import json

from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.models import Artifact, Flow, FlowArtifactEdge, FlowNode, FlowNodeEdge, Organization
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

        flow_nodes: list[FlowNode] = []
        for i, block in enumerate(flow_blocks):
            block = {**block}  # do not modify original

            try:
                name = block.pop("name")
                function_id = block.pop("function")
                config_arguments = {}
                if function_id == "bench.text.templatize":
                    config_arguments["template"] = block.pop("template")

                flow_node = FlowNode.objects.create(
                    name=name,
                    function_id=function_id,
                    config_arguments=config_arguments,
                    flow=flow_version,
                )

                # assume remaining arguments are artifact edges
                for argument_name, artifact_name in block.items():
                    artifact = Artifact.objects.get(name=artifact_name, organization=organization)
                    flow_node.connected_artifacts.add(
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
            if flow_nodes:
                previous_node = flow_nodes[-1]
                flow_node.depends_on_nodes.add(
                    previous_node,
                    through_defaults=dict(
                        flow=flow_version,
                        connection_type=FlowNodeEdge.ConnectionType.Input,
                    ),
                )
            flow_nodes.append(flow_node)

        self.stdout.write(self.style.SUCCESS(f"Updated flow with new version: {flow_version}"))
