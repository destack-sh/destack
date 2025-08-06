import inspect
import sys
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, get_args, get_origin

from . import console


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
    help: str | None = None
    commands: dict[str, CLICommand] = field(default_factory=dict)
    clis: dict[str, "CLI"] = field(default_factory=dict)

    def command(self, name: str | None = None):
        """Decorator to register a command."""

        def decorator(func: Callable):
            cmd_name = name or func.__name__.replace("_", "-")  # type: ignore
            self.commands[cmd_name] = CLICommand(func, cmd_name)
            return func

        return decorator

    def add_sub_cli(self, name: str, sub_cli: "CLI"):
        """Add a sub-CLI."""
        self.clis[name] = sub_cli
        sub_cli.name = name

    def run(self, argv: list[str] | None = None):
        """Run the CLI application."""
        args = argv or sys.argv[1:]

        if not args or args[0] in ["-h", "--help"]:
            self.show_help()
            return

        first_arg = args[0]

        # check if it's a sub-CLI
        if first_arg in self.clis:
            sub_cli = self.clis[first_arg]
            sub_cli.run(args[1:])
            return

        # check if it's a command
        if first_arg in self.commands:
            command = self.commands[first_arg]
            self.run_command(command, args[1:])
            return

        # unknown command/sub-CLI
        console.error(f"unknown command: `{first_arg}`")
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
            console.error(f"unexpected argument: {positionals[pos_idx]}")
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
            console.error(f"Error: {e}")
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
            console.print(self.help, "bold")
            console.print("")

        # show sub-CLIs first
        if self.clis:
            console.print("Sub-commands:", "cyan", "bold")
            max_name_len = max(len(name) for name in self.clis)
            for name, sub_cli in sorted(self.clis.items()):
                help_text = sub_cli.help or ""
                if help_text:
                    help_text = help_text.split("\n")[0]  # first line only
                console.print(
                    f"  {console.color(name, 'cyan')}{' ' * (max_name_len - len(name) + 2)}{help_text}",
                    "gray",
                )
            console.print("")

        # then show direct commands
        if self.commands:
            console.print("Commands:", "cyan", "bold")
            max_name_len = max(len(name) for name in self.commands)
            for name, cmd in sorted(self.commands.items()):
                help_text = cmd.help or ""
                if help_text:
                    help_text = help_text.split("\n")[0]  # first line only
                console.print(
                    f"  {console.color(name, 'cyan')}{' ' * (max_name_len - len(name) + 2)}{help_text}",
                    "gray",
                )
            console.print("")

        if self.commands or self.clis:
            console.print("Use '<command> --help' for more information about a command.", "dim")

    def show_command_help(self, command: CLICommand):
        """Show help for a specific command."""
        console.print(f"{command.name}", "cyan", "bold")
        if command.help:
            console.print(f"  {command.help}")

        sig = inspect.signature(command.func)
        params = list(sig.parameters.values())

        if params:
            console.print("")
            console.print("Arguments:", "yellow")
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

                console.print("".join(parts))


def create_cli(name: str | None = None, help: str | None = None) -> CLI:
    """Create a new CLI application."""
    return CLI(name, help)


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
    commands: dict[str, Callable] = field(default_factory=dict)
    help_texts: dict[str, str] = field(default_factory=dict)

    def command(self, name: str | None = None, help_text: str | None = None):
        """Decorator to register a REPL command."""

        def decorator(func: Callable):
            cmd_name = name or func.__name__.replace("_", "-")  # type: ignore
            self.commands[cmd_name] = func
            if help_text or func.__doc__:
                self.help_texts[cmd_name] = help_text or (func.__doc__ or "").strip()
            return func

        return decorator

    def run(self):
        """Run the REPL loop."""
        if self.welcome:
            console.print(self.welcome, "green", "bold")
            console.print("")

        while True:
            try:
                # get user input
                user_input = console.prompt(self.prompt_text.rstrip())

                if not user_input:
                    continue

                # check for exit
                if user_input.lower() in self.exit_commands:
                    console.print("Goodbye!", "dim")
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

                # find and execute command
                if cmd_name in self.commands:
                    try:
                        self.commands[cmd_name](*args)
                    except TypeError as e:
                        console.error(f"Error: {e}")
                        console.stacktrace(e)
                        if cmd_name in self.help_texts:
                            console.print(f"Usage: {self.help_texts[cmd_name]}", "dim")
                    except Exception as e:
                        console.error(f"Error executing '{cmd_name}': {e}")
                        console.stacktrace(e)
                else:
                    # try to pass the entire input to a default handler if it exists
                    if "default" in self.commands:
                        try:
                            self.commands["default"](user_input)
                        except Exception as e:
                            console.error(f"Error: {e}")
                            console.stacktrace(e)
                    else:
                        console.error(f"Unknown command: '{cmd_name}'")
                        console.print("Type 'help' for available commands.", "dim")

            except KeyboardInterrupt:
                console.print("\n" + console.color("Use 'exit' or 'quit' to leave.", "yellow"))
            except EOFError:
                console.print("\nGoodbye!", "dim")
                break

    def show_help(self, command: str | None = None):
        """Show help for commands."""
        if command and command in self.commands:
            # show help for specific command
            console.print(f"\n{console.color(command, 'cyan', 'bold')}")
            if command in self.help_texts:
                console.print(f"  {self.help_texts[command]}")
            else:
                console.print("  No documentation available.", "dim")
        else:
            # show all commands
            console.print("\nAvailable commands:", "cyan", "bold")
            console.print("")

            # built-in commands
            console.print(f"  {console.color('help, ?', 'yellow')}  - Show this help message")
            exit_cmds = ", ".join(self.exit_commands)
            console.print(f"  {console.color(exit_cmds, 'yellow')}  - Exit the program")

            if self.commands:
                console.print("")
                # user commands
                max_len = max(len(cmd) for cmd in self.commands if cmd != "default")
                for cmd_name in sorted(self.commands):
                    if cmd_name == "default":
                        continue
                    padding = " " * (max_len - len(cmd_name) + 2)
                    help_text = self.help_texts.get(cmd_name, "")
                    console.print(f"  {console.color(cmd_name, 'cyan')}{padding}{help_text}")
        console.print("")
