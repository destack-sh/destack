import typer

app = typer.Typer(short_help="bench-local resources")

pg = typer.Typer()
os = typer.Typer()
app.add_typer(pg, name="pg")
app.add_typer(os, name="os")
