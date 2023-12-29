from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.language.const import NodeType, StructType
from bench.language.module import (
    FINAL_BENCH_TYPES,
    NODE_CLASS_BY_NODE_TYPE,
    STRUCT_CLASS_BY_STRUCT_TYPE,
    Node,
)
from bench.proto.core import Field, Message
from bench.proto.wiring import generate_proto_schema


def write_schema(path: str, schema):
    with open(path, "w") as f:
        f.write(str(schema))


class Command(BaseCommand):
    help = "Manages the proto schema"

    def add_arguments(self, parser: CommandParser):
        # optional file name to write to (defaults to stdout)
        parser.add_argument("--file", type=str, required=False)

    @transaction.atomic
    def handle(self, *args, **options):
        struct_ts = [
            STRUCT_CLASS_BY_STRUCT_TYPE[t] for t in StructType if t in STRUCT_CLASS_BY_STRUCT_TYPE
        ]
        node_ts = [NODE_CLASS_BY_NODE_TYPE[t] for t in NodeType if t in NODE_CLASS_BY_NODE_TYPE]
        # :ProtoSchema
        proto = generate_proto_schema(
            bench_types=[*FINAL_BENCH_TYPES, Node],
            aliases={Node: "BaseNode"},
            unions={"SomeNode": ("node", node_ts), "SomeStruct": ("struct", struct_ts)},
            extras=[
                Message(
                    name="ModuleTree",
                    fields=[
                        Field(id=1, name="module", type="ModuleData"),
                        Field(id=2, name="nodes", type="SomeNodeData", repeated=True),
                    ],
                )
            ],
            message_postfix="Data",
        )
        schema_str = proto.to_proto_source()

        if options["file"]:
            write_schema(options["file"], schema_str)
        else:
            print(schema_str)
