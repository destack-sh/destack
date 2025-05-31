import asyncio
import sys


async def kill_process_on_port(port: int) -> None:
    """Kills any process listening on a given port."""
    if sys.platform == "win32":
        # on Windows, use netstat to find and kill process
        proc = await asyncio.create_subprocess_exec(
            "netstat",
            "-ano",
            "-p",
            "TCP",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, _ = await proc.communicate()
        for line in stdout.decode().splitlines():
            if f":{port}" in line and "LISTENING" in line:
                pid = line.strip().split()[-1]
                await asyncio.create_subprocess_exec("taskkill", "/F", "/PID", pid)
                break
    else:
        # on Unix, use lsof to find and kill process
        proc = await asyncio.create_subprocess_exec(
            "lsof",
            "-i",
            f":{port}",
            "-t",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, _ = await proc.communicate()
        if stdout:
            pid = stdout.decode().strip()
            await asyncio.create_subprocess_exec("kill", "-9", pid)
