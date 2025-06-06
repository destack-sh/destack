from typing import Any

from bench.language import Client, CustomNodeInstance, IsSubject, Session

# nocheckin: Scripts
session: Any = ...
script: Any = ...

secret_key = script.member("secret_key", str)


def action(func):
    return func


@action
async def do_something(
    session: Session,
    subject: IsSubject,
    client: Client,
    event: CustomNodeInstance,
):
    pass


@action
def do_something_else(
    session: Session,
    subject: IsSubject,
    client: Client,
    event: CustomNodeInstance,
):
    pass
