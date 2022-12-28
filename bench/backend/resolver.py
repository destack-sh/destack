from __future__ import annotations

from bench import language, models


class Resolver:
    """
    Server-side resolver to remotely read and write files and statements.
    Keeps track of revisions used for automatic caching and re-computation.
    """

    def read_statement(self, statement: models.Statement) -> language.Statement:
        raise NotImplementedError
