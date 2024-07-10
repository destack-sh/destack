import contextlib
import io
from typing import Any, Iterable, override

from bench.language.log import LogInfo, LogLevel
from bench.language.text import Text
from bench.utils.oracle import Oracle


@contextlib.contextmanager
def capture_stdout():
    with contextlib.redirect_stdout(io.StringIO()) as buffer:
        yield buffer


@contextlib.contextmanager
def capture_stderr():
    with contextlib.redirect_stderr(io.StringIO()) as buffer:
        yield buffer


TextIn = str | Text | Any


class LogSink:
    """A sink for capturing logs."""

    __slots__ = ["logs", "oracle"]

    def __init__(self, oracle: Oracle):
        self.oracle = oracle
        self.logs: list[LogInfo] = []

    def _capture(self, *, level: LogLevel, text_in: TextIn, **kwargs) -> LogInfo:
        # NOTE :Incomplete: transform kwargs into freeform LogInfo.values
        if isinstance(text_in, str):
            text = None
            text_plain = text_in.strip()
        elif isinstance(text_in, Text):
            text = text_in
            text_plain = None
        else:
            text = None
            text_plain = repr(text_in)
        if text_plain is not None and len(text_plain) == 0:
            text_plain = None  # we can't have empty strings
        log = LogInfo(created_at=self.oracle.utc(), level=level, text=text, text_plain=text_plain)
        self.logs.append(log)
        return log

    def bind(self, **kwargs) -> "BoundLogSink":
        """Bind additional kwargs to this log sink."""
        return BoundLogSink(self, kwargs)

    #
    # Log methods
    #

    def log(self, _arg1: TextIn | LogLevel, _arg2: TextIn | LogLevel = LogLevel.INFO, **kwargs):
        """Log a message at the given level with optional kwargs."""
        if isinstance(_arg1, LogLevel):
            level = _arg1
            assert not isinstance(_arg2, LogLevel), "cannot have two LogLevel arguments"
            text = _arg2
        else:
            assert isinstance(_arg2, LogLevel), "second argument must be LogLevel if first isn't"
            level = _arg2
            text = _arg1
        return self._capture(level=level, text_in=text, **kwargs)

    __call__ = log

    def trace(self, text: TextIn, **kwargs) -> LogInfo:
        """Log a trace message."""
        return self._capture(level=LogLevel.TRACE, text_in=text, **kwargs)

    def debug(self, text: TextIn, **kwargs) -> LogInfo:
        """Log a debug message."""
        return self._capture(level=LogLevel.DEBUG, text_in=text, **kwargs)

    def info(self, text: TextIn, **kwargs) -> LogInfo:
        """Log an info message."""
        return self._capture(level=LogLevel.INFO, text_in=text, **kwargs)

    def warn(self, text: TextIn, **kwargs) -> LogInfo:
        """Log a warning message."""
        return self._capture(level=LogLevel.WARNING, text_in=text, **kwargs)

    warning = warn

    def error(self, text: TextIn, **kwargs) -> LogInfo:
        """Log an error message."""
        return self._capture(level=LogLevel.ERROR, text_in=text, **kwargs)

    def critical(self, text: TextIn, **kwargs) -> LogInfo:
        """Log a critical message."""
        return self._capture(level=LogLevel.CRITICAL, text_in=text, **kwargs)

    #
    # Print wrapper
    # We try to support most print arguments and put an entire print into one log.
    #

    def print(
        self,
        *args,
        sep=" ",
        end="\n",
        file=None,
        flush=False,
        level: LogLevel = LogLevel.INFO,
        **kwargs,
    ) -> LogInfo:
        """Print to the log."""
        assert file is None, "cannot specify file for print"
        assert not flush, "cannot specify flush for print"
        text = sep.join(repr(arg) if not isinstance(arg, str) else arg for arg in args) + end
        return self._capture(level=level, text_in=text, **kwargs)


class BoundLogSink(LogSink):
    """A log sink with bound kwargs."""

    def __init__(self, sink: LogSink, kwargs: dict[str, Any]):
        self.sink = sink
        self.kwargs = kwargs

    @override
    def bind(self, **kwargs) -> "BoundLogSink":
        combined_kwargs = {**self.kwargs}
        combined_kwargs.update(kwargs)
        return BoundLogSink(self.sink, combined_kwargs)

    @override
    def _capture(self, *, level: LogLevel, text_in: TextIn, **kwargs) -> LogInfo:
        combined_kwargs = {**self.kwargs}
        combined_kwargs.update(kwargs)
        return self.sink._capture(level=level, text_in=text_in, **combined_kwargs)


class LogStringIO(io.StringIO):
    """
    Transform text written to this buffer into captured logs.
    We override log/print/debug/etc. in code context, so this captures any output
     that wasn't logged through that interface - so we do a best effort to make nice logs.
    """

    def __init__(self, sink: LogSink, level: LogLevel):
        super().__init__()
        self.sink = sink
        self.level = level
        self._current_line_parts: list[str] = []

    def write(self, s: str) -> int:
        # accumulate s into current line until we find a newline (if it doesn't have one)
        if "\n" in s:
            if len(s) > 1 or self._current_line_parts:
                line = "\n".join(self._current_line_parts) + s.strip()
                self._current_line_parts.clear()
                self.sink.log(self.level, line)
        elif s:
            self._current_line_parts.append(s)
        return super().write(s)

    def writelines(self, lines: Iterable[str]):
        self.sink.log(self.level, "\n".join(lines))
        return super().writelines(lines)


@contextlib.contextmanager
def capture_logs(sink: LogSink):
    """Redirect stdout/stderr to the given log sink."""
    stdout_sink = LogStringIO(sink, LogLevel.INFO)
    stderr_sink = LogStringIO(sink, LogLevel.ERROR)
    with contextlib.redirect_stdout(stdout_sink), contextlib.redirect_stderr(stderr_sink):
        yield
