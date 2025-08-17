from destack.core.local import CLI, _generate, _manual, _version, create_cli

# create main CLI app
cli = create_cli(help="Destack CLI")

# register selected sub-CLIs
sub_clis: list[tuple[str, CLI]] = [
    ("version", _version.cli),
    ("manual", _manual.cli),
    ("generate", _generate.cli),
]
for name, sub_cli in sub_clis:
    cli.add_sub_cli(name, sub_cli)

if __name__ == "__main__":
    cli.run()
