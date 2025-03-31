import abc
from uuid import UUID

from bench.language import Computer, GraphCapture
from bench.pb2.computer_grpc import ComputerClient


class RuntimeScope:
    """
    A runtime object with a lifetime.
    """

    __slots__ = ("_computer_clients_by_uri", "capture", "id")

    def __init__(self, id: UUID, capture: GraphCapture | None = None):
        self.id = id
        self.capture = capture
        self._computer_clients_by_uri: dict[str, ComputerClient] = {}

    async def get_computer_client(self, computer: Computer) -> ComputerClient:
        """Get a ComputerClient for the given Computer and display."""
        raise NotImplementedError(f"nocheckin: get {computer!r} service from {self!r}")

    @abc.abstractmethod
    def close(self):
        """Close the scope."""
        if self.capture is not None:
            self.capture.close_and_detach()
