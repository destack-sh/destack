from dataclasses import dataclass

import zmq


@dataclass
class ZMessage:
    version: int


zmq_ctx = zmq.Context()
