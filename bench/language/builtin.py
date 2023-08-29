import os
from uuid import uuid5

from bench.language.basic import BENCH_UUID_NAMESPACE
from bench.language.core import Module

#
# Common base for builtin libraries for reference outside libs.
# (libs requires more imports than just the basics)
#


def _make_lib_module(name: str) -> Module:
    ck = uuid5(BENCH_UUID_NAMESPACE, f"builtin:{name}")
    id = uuid5(ck, os.environ["VERSION"])
    return Module(name=name, ck=ck, id=id)


symbolx_lib = _make_lib_module("symbolx.lib")
openai_lib = _make_lib_module("openai.lib")
anthropic_lib = _make_lib_module("anthropic.lib")
