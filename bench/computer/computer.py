import asyncio
import base64
import os
from typing import Mapping
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.pb2 import (
    ClickRequest,
    ComputerBase,
    Empty,
    MoveRequest,
    PressRequest,
    ScreenshotRequest,
    ScreenshotResponse,
    ScrollRequest,
    ServiceKind,
    ShellCommandRequest,
    ShellCommandResponse,
    TypeRequest,
)
from bench.proto import Network, ServiceBase
from bench.utils.oracle import Oracle

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class ComputerService(ServiceBase, ComputerBase):
    kind = ServiceKind.INTERNAL

    def __init__(
        self, id: str, network: Network, oracle: Oracle, computer_id: UUID, display: str | None
    ):
        super().__init__(id=id, logger=logger, tracer=tracer, network=network, oracle=oracle)
        self.computer_id = computer_id
        self.display = display

    @tracer.start_as_current_span("computer.shell")
    async def _execute_shell(self, cmd: str) -> tuple[str, str, int]:
        """Execute a shell command and return stdout, stderr, and return code"""
        try:
            logger.trace("computer.shell.start", cmd=cmd)
            env = {**os.environ}
            if self.display:
                env["DISPLAY"] = f":{self.display}"
            proc = await asyncio.create_subprocess_shell(
                cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                env=env,
            )
            stdout, stderr = await proc.communicate()
            stdout = stdout.decode()
            stderr = stderr.decode()
            self.logger.debug(
                "computer.shell",
                span="current",
                cmd=cmd,
                return_code=proc.returncode,
                stdout_len=len(stdout),
                stderr_len=len(stderr),
            )
            return stdout, stderr, proc.returncode or 0
        except Exception as e:
            self.logger.error("computer.shell.error", cmd=cmd, exc_info=e)
            raise

    @tracer.start_as_current_span("computer.exec")
    async def _execute_cmd(self, program: str, *args: str) -> tuple[bytes, bytes, int]:
        """Execute a command with arguments and return stdout, stderr, and return code"""
        try:
            logger.trace("computer.exec.start", program=program, args=args)
            proc = await asyncio.create_subprocess_exec(
                program,
                *args,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                env={**os.environ, "DISPLAY": f":{self.display}"},
            )
            stdout, stderr = await proc.communicate()
            self.logger.debug(
                "computer.exec",
                span="current",
                program=program,
                args=args,
                return_code=proc.returncode,
                stdout_len=len(stdout),
                stderr_len=len(stderr),
            )
            return stdout, stderr, proc.returncode or 0
        except Exception as e:
            self.logger.error("computer.exec.error", program=program, args=args, exc_info=e)
            raise

    #
    # Desktop
    #

    async def screenshot(self, request: ScreenshotRequest, headers: Mapping) -> ScreenshotResponse:
        """Take a screenshot of the current screen"""
        cmd = "import -window root jpeg:- | base64 -w 0"
        image_b64, _, _ = await self._execute_shell(cmd)
        image_bytes = base64.b64decode(image_b64)
        return ScreenshotResponse(image=image_bytes)

    async def click(self, request: ClickRequest, headers: Mapping) -> Empty:
        """Click an element at the specified coordinates"""
        button = request.button or "left"
        await self._execute_cmd(
            "xdotool", "mousemove", str(request.x), str(request.y), "click", button
        )
        return Empty()

    async def double_click(self, request: ClickRequest, headers: Mapping) -> Empty:
        """Double click an element at the specified coordinates"""
        button = request.button or "left"
        await self._execute_cmd(
            "xdotool", "mousemove", str(request.x), str(request.y), "click", "--repeat", "2", button
        )
        return Empty()

    async def press(self, request: PressRequest, headers: Mapping) -> Empty:
        """Press the specified keys"""
        key_sequence = " ".join(request.keys)
        await self._execute_cmd("xdotool", "key", key_sequence)
        return Empty()

    async def type(self, request: TypeRequest, headers: Mapping) -> Empty:
        """Type the specified text"""
        await self._execute_cmd("xdotool", "type", request.text)
        return Empty()

    async def move(self, request: MoveRequest, headers: Mapping) -> Empty:
        """Move the mouse to the specified coordinates"""
        await self._execute_cmd("xdotool", "mousemove", str(request.x), str(request.y))
        return Empty()

    async def scroll(self, request: ScrollRequest, headers: Mapping) -> Empty:
        """Scroll the mouse at the specified coordinates"""
        # first move to the specified position
        await self._execute_cmd("xdotool", "mousemove", str(request.x), str(request.y))
        # then scroll
        if request.scroll_y != 0:
            direction = "up" if request.scroll_y < 0 else "down"
            clicks = abs(request.scroll_y)
            await self._execute_cmd("xdotool", "click", "--repeat", str(clicks), direction)
        # then scroll horizontally
        if request.scroll_x != 0:
            direction = "left" if request.scroll_x < 0 else "right"
            clicks = abs(request.scroll_x)
            await self._execute_cmd("xdotool", "click", "--repeat", str(clicks), direction)
        return Empty()

    #
    # Terminal
    #

    # TODO :Incomplete! :Architecture: Computer shell / "Jupyter" / browsing / :ComputerUse ...

    async def shell(self, request: ShellCommandRequest, headers: Mapping) -> ShellCommandResponse:
        """Execute a shell command and return its output"""
        stdout, stderr, returncode = await self._execute_shell(request.command)
        return ShellCommandResponse(exit_code=returncode, stdout=stdout, stderr=stderr)
