import contextlib
import io
from typing import Any, Iterable, override

from bench.language import LogInfo, LogLevel, Text
from bench.utils.oracle import Oracle
from bench.utils.utils import get_from_env

TextIn = str | Text | Any

MAX_LOGS_PER_CAPTURE = get_from_env(
    "MAX_LOGS_PER_CAPTURE",
    typ=int,
    default=100,
    description="Maximum Logs to capture per capture (i.e. run)",
)
MAX_LOG_LINE_LENGTH = get_from_env(
    "MAX_LOG_LINE_LENGTH",
    typ=int,
    default=1000,
    description="Maximum length of a log line in characters",
)


class LogSink:
    """A sink for capturing logs."""

    __slots__ = ["is_open", "logs", "max_log_length", "max_logs", "oracle"]

    def __init__(
        self,
        oracle: Oracle,
        max_logs: int,
        max_log_length: int,
    ):
        self.oracle = oracle
        self.logs: list[LogInfo] = []
        self.max_logs = max_logs
        self.is_open = len(self.logs) < self.max_logs
        self.max_log_length = max_log_length

    def _capture(self, *, level: LogLevel, text_in: TextIn, **kwargs):
        if not self.is_open:
            return

        # coerce
        if isinstance(text_in, str):
            text = None
            text_plain = text_in.strip()
        elif isinstance(text_in, Text):
            text = text_in
            text_plain = None
        else:
            text = None
            text_plain = repr(text_in)

        # coerce to none if empty
        if text_plain is not None and len(text_plain) == 0:
            text_plain = None  # we can't have empty strings

        # truncate plain text if too long
        if text_plain is not None and len(text_plain) > self.max_log_length:
            text_plain = text_plain[: self.max_log_length] + "... <line too long, truncated>"

        # NOTE :Incomplete: transform kwargs into freeform LogInfo.values
        log = LogInfo(
            created_at=self.oracle.utc(),
            level=level,
            text=text,
            text_plain=text_plain,
            _skip_validate_self=True,
        )
        self.logs.append(log)

        # close if overflown
        if self.is_open and len(self.logs) >= self.max_logs - 1:
            self.is_open = False
            self.logs.append(
                LogInfo(
                    created_at=self.oracle.utc(),
                    level=LogLevel.WARNING,
                    text_plain=f"<stopping log capture, log overflow (exceeded {self.max_logs} logs)>",
                    _skip_validate_self=True,
                )
            )

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
        self._capture(level=level, text_in=text, **kwargs)

    __call__ = log

    def trace(self, text: TextIn, **kwargs):
        """Log a trace message."""
        self._capture(level=LogLevel.TRACE, text_in=text, **kwargs)

    def debug(self, text: TextIn, **kwargs):
        """Log a debug message."""
        self._capture(level=LogLevel.DEBUG, text_in=text, **kwargs)

    def info(self, text: TextIn, **kwargs):
        """Log an info message."""
        self._capture(level=LogLevel.INFO, text_in=text, **kwargs)

    def warn(self, text: TextIn, **kwargs):
        """Log a warning message."""
        self._capture(level=LogLevel.WARNING, text_in=text, **kwargs)

    warning = warn

    def error(self, text: TextIn, **kwargs):
        """Log an error message."""
        self._capture(level=LogLevel.ERROR, text_in=text, **kwargs)

    def critical(self, text: TextIn, **kwargs):
        """Log a critical message."""
        self._capture(level=LogLevel.CRITICAL, text_in=text, **kwargs)

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
    ):
        """Print to the log."""
        assert file is None, "cannot specify file for print"
        assert not flush, "cannot specify flush for print"
        text = sep.join(repr(arg) if not isinstance(arg, str) else arg for arg in args) + end
        self._capture(level=level, text_in=text, **kwargs)


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
    def _capture(self, *, level: LogLevel, text_in: TextIn, **kwargs):
        combined_kwargs = {**self.kwargs}
        combined_kwargs.update(kwargs)
        self.sink._capture(level=level, text_in=text_in, **combined_kwargs)


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
