from typing import TYPE_CHECKING

if TYPE_CHECKING:
    pass


class Logger:
    __slots__ = ("name",)

    def __init__(self, name: str):
        self.name = name

    def trace(self, msg: str, *args, **kwargs):
        pass

    def debug(self, msg: str, *args, **kwargs):
        pass

    def info(self, msg: str, *args, **kwargs):
        pass

    def warning(self, msg: str, *args, **kwargs):
        pass

    def error(self, msg: str, *args, **kwargs):
        pass

    def critical(self, msg: str, *args, **kwargs):
        pass
