import subprocess


def _shell(cmd: str, check=True, **kwargs):
    print(f"shell: {cmd}")
    subprocess.run(cmd, shell=True, check=check, **kwargs)
