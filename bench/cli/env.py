import os
import re
from dataclasses import dataclass
from pathlib import Path

import structlog
import typer
from rich.console import Console
from rich.table import Table

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="environment variables")


@dataclass
class EnvDeclaration:
    key: str
    is_required: bool
    description: str | None
    default: str | None
    typ: str


def parse_env_var_site(site: str):
    key = description = default = typ = None
    site = site.replace("\n", " ")

    key_match = re.search(r'get_from_env(?:_maybe)?\s*\(\s*["](.*?)["]', site)
    if key_match:  # noqa: SIM108
        key = key_match.group(1)
    else:
        key = ""  # mark as invalid

    description_match = re.search(r'description\s*=\s*["](.*?)["]', site, re.DOTALL)
    description = description_match.group(1) if description_match else None

    default_match = re.search(r"default\s*=\s*([^,\(\)]+)", site)
    default = default_match.group(1).strip() if default_match else None
    is_required = "get_from_env(" in site and not default_match

    typ_match = re.search(r"typ\s*=\s*([^,\(\)]+)", site)
    typ = typ_match.group(1).strip() if typ_match else "str"

    return EnvDeclaration(key, is_required, description, default, typ)


def extract_env_vars_from_file(content: str):
    env_var_declarations = re.findall(r"get_from_env(?:_maybe)?\([\s\S]*?\)", content)
    parsed_declarations = [parse_env_var_site(call) for call in env_var_declarations]
    return [decl for decl in parsed_declarations if decl.key]


@app.command()
def show(path: str = "bench", current: bool = False):
    # collect env vars
    env_vars: dict[str, EnvDeclaration] = {}
    for f in Path(path).rglob("*.py"):
        file_text = Path(f).read_text()
        env_vars.update({decl.key: decl for decl in extract_env_vars_from_file(file_text)})

    # build table
    console = Console()
    table = Table(show_header=True, header_style="bold magenta")
    table.add_column("Name", style="bold cyan")
    table.add_column("Type", style="bold")
    table.add_column("Required", style="bold")
    table.add_column("Description", style="dim")
    table.add_column("Default", style="yellow")
    if current:
        table.add_column("Current", style="green")

    for key, details in env_vars.items():
        row = [
            key,
            details.typ,
            "✓" if details.is_required else "",
            details.description,
            details.default,
        ]
        if current:
            row.append(os.getenv(key, ""))
        table.add_row(*row)

    console.print(table)
