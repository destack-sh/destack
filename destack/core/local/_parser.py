import inspect
import readline
import sys
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, get_args, get_origin

from . import _console
import contextlib


@dataclass(slots=True)
class CLICommand:
    """Represents a CLI command."""

    func: Callable
    name: str
    help: str | None = None

    def __post_init__(self):
        if self.help is None:
            self.help = (inspect.getdoc(self.func) or "").replace("\n", " ").strip()


@dataclass(slots=True)
class CLI:
    """CLI application with Commands and sub-CLIs."""

    name: str | None = None
    aliases: list[str] = field(default_factory=list)
    help: str | None = None
    commands: list[CLICommand] = field(default_factory=list)
    clis: list["CLI"] = field(default_factory=list)

    def command(self, name: str | None = None):
        """Decorator to register a command."""

        def decorator(func: Callable):
            cmd_name = name or func.__name__.replace("_", "-")  # type: ignore
            self.commands.append(CLICommand(func, cmd_name))
            return func

        return decorator

    def add_sub_cli(self, name: str, sub_cli: "CLI"):
        """Add a sub-CLI."""
        # check for conflicts
        for existing_cli in self.clis:
            if existing_cli.name == name or name in existing_cli.aliases:
                raise ValueError(f"sub-CLI '{name}' already exists")
            if existing_cli.name in sub_cli.aliases:
                raise ValueError(f"sub-CLI '{existing_cli.name}' conflicts with alias")

        sub_cli.name = name
        self.clis.append(sub_cli)

    def run(self, argv: list[str] | None = None):
        """Run the CLI application."""
        args = argv or sys.argv[1:]

        if not args or args[0] in ["-h", "--help"]:
            self.show_help()
            return

        first_arg = args[0]

        # check if it's a sub-CLI
        for sub_cli in self.clis:
            if sub_cli.name == first_arg or first_arg in sub_cli.aliases:
                sub_cli.run(args[1:])
                return

        # check if it's a command
        for command in self.commands:
            if command.name == first_arg:
                self.run_command(command, args[1:])
                return

        # unknown command/sub-CLI
        _console.error(f"unknown command: `{first_arg}`")
        self.show_help()
        sys.exit(1)

    def run_command(self, command: CLICommand, args: list[str]):
        """Parse arguments and run a command."""
        sig = inspect.signature(command.func)
        parsed_args: dict[str, Any] = {}
        positionals: list[str] = []

        # parse arguments
        params = list(sig.parameters.values())
        i = 0
        while i < len(args):
            arg = args[i]

            if arg in ["-h", "--help"]:
                self.show_command_help(command)
                return

            if arg.startswith("--"):
                # long option
                if "=" in arg:
                    key, value = arg[2:].split("=", 1)
                else:
                    key = arg[2:]
                    value = (
                        args[i + 1]
                        if i + 1 < len(args) and not args[i + 1].startswith("-")
                        else None
                    )
                    if value is not None:
                        i += 1

                # find parameter
                param = None
                for p in params:
                    if p.name.replace("_", "-") == key:
                        param = p
                        break

                if param:
                    parsed_args[param.name] = self.parse_value(value, param)
            elif arg.startswith("-") and len(arg) == 2:
                # short option
                key = arg[1]
                value = (
                    args[i + 1] if i + 1 < len(args) and not args[i + 1].startswith("-") else None
                )
                if value is not None:
                    i += 1

                # find parameter by first letter
                param = None
                for p in params:
                    if p.name[0] == key:
                        param = p
                        break

                if param:
                    parsed_args[param.name] = self.parse_value(value, param)
            else:
                # positional argument
                positionals.append(arg)

            i += 1

        # map positionals to parameters (only required parameters without defaults)
        pos_idx = 0
        for param in params:
            # only consume positionals for required parameters (no default value)
            if (
                param.name not in parsed_args
                and param.default is param.empty
                and pos_idx < len(positionals)
            ):
                parsed_args[param.name] = self.parse_value(positionals[pos_idx], param)
                pos_idx += 1

        # check for unused positional arguments
        if pos_idx < len(positionals):
            _console.error(f"unexpected argument: {positionals[pos_idx]}")
            self.show_command_help(command)
            sys.exit(1)

        # apply defaults
        for param in params:
            if param.name not in parsed_args:
                if param.default is not param.empty:
                    parsed_args[param.name] = param.default
                elif param.annotation is bool:
                    parsed_args[param.name] = False

        # call the function
        try:
            command.func(**parsed_args)
        except TypeError as e:
            _console.error(f"Error: {e}")
            self.show_command_help(command)
            sys.exit(1)

    def parse_value(self, value: str | None, param: inspect.Parameter) -> Any:
        """Parse a value based on parameter type annotation."""
        if param.annotation is param.empty:
            return value
        # handle bool specially
        if param.annotation is bool:
            if value is None:
                return True
            return value.lower() in ["true", "yes", "1", "on"]
        # handle None/optional types
        origin = get_origin(param.annotation)
        args = get_args(param.annotation)
        # check if it's a union type (e.g., int | None)
        if origin is type(type(None)) and type(None) in args:
            # it's an Optional type, find the non-None type
            for arg in args:
                if arg is not type(None):
                    return self.parse_simple_type(value, arg)
            return None
        # check for other union types
        if args and len(args) > 1:
            # try to parse as the first non-None type
            for arg in args:
                if arg is not type(None):
                    return self.parse_simple_type(value, arg)
            return value
        return self.parse_simple_type(value, param.annotation)

    def parse_simple_type(self, value: str | None, typ: type) -> Any:
        """Parse a simple primitive type."""
        if value is None:
            return None

        if typ is int:
            return int(value)
        elif typ is float:
            return float(value)
        elif typ is Path:
            return Path(value)
        elif typ is bool:
            return value.lower() in ["true", "yes", "1", "on"]
        else:
            return value

    def show_help(self):
        """Show general help."""
        if self.help:
            _console.print(self.help, "bold")
            _console.print("")

        # show sub-CLIs first
        if self.clis:
            _console.print("CLIs:", "cyan", "bold")
            max_name_len = max(len(cli.name or "") for cli in self.clis)
            for sub_cli in sorted(self.clis, key=lambda c: c.name or ""):
                name = sub_cli.name or ""
                help_text = sub_cli.help or ""
                if help_text:
                    help_text = help_text.split("\n")[0]  # first line only
                aliases_text = ""
                if sub_cli.aliases:
                    aliases_text = f" (aliases: {', '.join(sub_cli.aliases)})"
                _console.print(
                    f"  {_console.color(name, 'cyan')}{' ' * (max_name_len - len(name) + 2)}{help_text}{aliases_text}",
                    "gray",
                )
            _console.print("")

        # then show direct commands
        if self.commands:
            _console.print("Commands:", "cyan", "bold")
            max_name_len = max(len(cmd.name) for cmd in self.commands)
            for cmd in sorted(self.commands, key=lambda c: c.name):
                help_text = cmd.help or ""
                if help_text:
                    help_text = help_text.split("\n")[0]  # first line only
                _console.print(
                    f"  {_console.color(cmd.name, 'cyan')}{' ' * (max_name_len - len(cmd.name) + 2)}{help_text}",
                    "gray",
                )
            _console.print("")

        if self.commands or self.clis:
            _console.print("Use '<command> --help' for more information about a command.", "dim")

    def show_command_help(self, command: CLICommand):
        """Show help for a specific command."""
        _console.print(f"{command.name}", "cyan", "bold")
        if command.help:
            _console.print(f"  {command.help}")

        sig = inspect.signature(command.func)
        params = list(sig.parameters.values())

        if params:
            _console.print("")
            _console.print("Arguments:", "yellow")
            for param in params:
                param_name = param.name.replace("_", "-")

                # build parameter description
                parts = [f"  --{param_name}"]
                if len(param.name) > 1:
                    parts.append(f" (-{param.name[0]})")

                # add type hint
                if param.annotation is not param.empty:
                    type_str = getattr(param.annotation, "__name__", str(param.annotation))
                    parts.append(f" [{type_str}]")

                # add default value
                if param.default is not param.empty:
                    parts.append(f" (default: {param.default})")

                _console.print("".join(parts))


def create_cli(
    name: str | None = None,
    *,
    aliases: list[str] | None = None,
    help: str | None = None,
) -> CLI:
    """Create a new CLI application."""
    return CLI(
        name=name,
        aliases=aliases or [],
        help=help,
    )


def create_repl(
    prompt_text: str = "> ",
    welcome: str | None = None,
    exit_commands: list[str] | None = None,
) -> "REPL":
    """Create a new REPL (Read-Eval-Print Loop) application."""
    return REPL(prompt_text, welcome, exit_commands or ["exit", "quit", "q"])


@dataclass(slots=True)
class REPL:
    """Interactive REPL application."""

    prompt_text: str = "> "
    welcome: str | None = None
    exit_commands: list[str] = field(default_factory=lambda: ["exit", "quit", "q"])
    commands: list[CLICommand] = field(default_factory=list)
    clis: list[CLI] = field(default_factory=list)

    def command(self, name: str | None = None, help_text: str | None = None):
        """Decorator to register a REPL command."""

        def decorator(func: Callable):
            cmd_name = name or func.__name__.replace("_", "-")  # type: ignore
            cmd = CLICommand(func, cmd_name, help_text)
            self.commands.append(cmd)
            return func

        return decorator

    def add_sub_cli(self, name: str, sub_cli: CLI):
        """Add a sub-CLI."""
        # check for conflicts
        for existing_cli in self.clis:
            if existing_cli.name == name or name in existing_cli.aliases:
                raise ValueError(f"sub-CLI '{name}' already exists")
            if existing_cli.name in sub_cli.aliases:
                raise ValueError(f"sub-CLI '{existing_cli.name}' conflicts with alias")

        sub_cli.name = name
        self.clis.append(sub_cli)

    def run(self):
        """Run the REPL loop."""
        # enable up/down history for both libedit (macOS) and GNU Readline
        try:
            impl = readline.__doc__ or ""
            if "libedit" in impl:
                # map to simple prev/next history only (no custom arrow bindings)
                readline.parse_and_bind("bind ^P ed-prev-history")
                readline.parse_and_bind("bind ^N ed-next-history")
                # unbind any stray single-letter bindings that could have been set
                for key in ("e", "E"):
                    with contextlib.suppress(Exception):
                        readline.parse_and_bind(f"bind {key} self-insert")
            else:
                # standard GNU readline: basic history navigation on arrows
                readline.parse_and_bind(r"\e[A: previous-history")
                readline.parse_and_bind(r"\e[B: next-history")
        except Exception:
            pass
        if self.welcome:
            _console.print(self.welcome, "green", "bold")
            _console.print("")

        while True:
            try:
                # get user input
                user_input = _console.prompt(self.prompt_text.rstrip())
                # push into history so up/down navigation works
                if user_input:
                    readline.add_history(user_input)

                if not user_input:
                    continue

                # check for exit
                if user_input.lower() in self.exit_commands:
                    break

                # parse command and arguments
                parts = user_input.split()
                if not parts:
                    continue

                cmd_name = parts[0]
                args = parts[1:]

                # handle help
                if cmd_name in ["help", "?"]:
                    self.show_help(args[0] if args else None)
                    continue

                # check if it's a sub-CLI
                for sub_cli in self.clis:
                    if sub_cli.name == cmd_name or cmd_name in sub_cli.aliases:
                        sub_cli.run(args)
                        continue

                # find and execute command
                command = None
                for cmd in self.commands:
                    if cmd.name == cmd_name:
                        command = cmd
                        break

                if command:
                    try:
                        command.func(*args)
                    except TypeError as e:
                        _console.error(f"Error: {e}")
                        _console.stacktrace(e)
                        if command.help:
                            _console.print(f"Usage: {command.help}", "dim")
                    except Exception as e:
                        _console.error(f"Error executing '{cmd_name}': {e}")
                        _console.stacktrace(e)
                else:
                    # try to pass the entire input to a default handler if it exists
                    default_command = None
                    for cmd in self.commands:
                        if cmd.name == "default":
                            default_command = cmd
                            break
                    if default_command:
                        try:
                            default_command.func(user_input)
                        except Exception as e:
                            _console.error(f"Error: {e}")
                            _console.stacktrace(e)
                    else:
                        _console.error(f"Unknown command: '{cmd_name}'")
                        _console.print("Type 'help' for available commands.", "dim")

            except KeyboardInterrupt:
                _console.print("\n" + _console.color("Use 'exit' or 'quit' to leave.", "yellow"))
            except EOFError:
                _console.print("\nGoodbye!", "dim")
                break

    def show_help(self, command: str | None = None):
        """Show help for commands."""
        if command:
            # look for specific command
            found_command = None
            for cmd in self.commands:
                if cmd.name == command:
                    found_command = cmd
                    break

            # look for sub-CLI
            found_cli = None
            for cli in self.clis:
                if cli.name == command or command in cli.aliases:
                    found_cli = cli
                    break

            if found_command:
                _console.print(f"\n{_console.color(command, 'cyan', 'bold')}")
                if found_command.help:
                    _console.print(f"  {found_command.help}")
                else:
                    _console.print("  No documentation available.", "dim")
            elif found_cli:
                found_cli.show_help()
            else:
                _console.error(f"Unknown command: '{command}'")
        else:
            # show all commands
            _console.print("\nAvailable commands:", "cyan", "bold")
            _console.print("")

            # built-in commands
            _console.print(f"  {_console.color('help, ?', 'yellow')}  - Show this help message")
            exit_cmds = ", ".join(self.exit_commands)
            _console.print(f"  {_console.color(exit_cmds, 'yellow')}  - Exit the program")

            # sub-CLIs
            if self.clis:
                _console.print("")
                max_name_len = max(len(cli.name or "") for cli in self.clis)
                for cli in sorted(self.clis, key=lambda c: c.name or ""):
                    name = cli.name or ""
                    help_text = cli.help or ""
                    if help_text:
                        help_text = help_text.split("\n")[0]  # first line only
                    aliases_text = ""
                    if cli.aliases:
                        aliases_text = f" (aliases: {', '.join(cli.aliases)})"
                    _console.print(
                        f"  {_console.color(name, 'cyan')}{' ' * (max_name_len - len(name) + 2)}{help_text}{aliases_text}",
                        "gray",
                    )

            # user commands
            if self.commands:
                _console.print("")
                max_len = max(len(cmd.name) for cmd in self.commands if cmd.name != "default")
                for cmd in sorted(self.commands, key=lambda c: c.name):
                    if cmd.name == "default":
                        continue
                    padding = " " * (max_len - len(cmd.name) + 2)
                    help_text = cmd.help or ""
                    _console.print(f"  {_console.color(cmd.name, 'cyan')}{padding}{help_text}")
        _console.print("")
