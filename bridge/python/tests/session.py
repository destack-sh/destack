import tempfile
from dataclasses import dataclass

from destack import Edit, Repository, Source, Workspace


@dataclass
class RepositoryFixture:
    root: str
    repository: Repository


@dataclass
class WorkspaceFixture:
    root: str
    workspace: Workspace

    def relative_uri(self, uri: str) -> str:
        if uri.startswith(self.root):
            return uri[len(self.root) :].lstrip("/")

        return uri


def open_workspace(files: list[tuple[str, str]]) -> WorkspaceFixture:
    fixture = open_repository(files)
    workspace = fixture.repository.workspace()

    return WorkspaceFixture(fixture.root, workspace)


def open_repository(files: list[tuple[str, str]]) -> RepositoryFixture:
    root = tempfile.mkdtemp(prefix="destack-bridge-python-tests-")

    source = Source.memory(
        root,
        [Edit.set_text(path, text) for path, text in files],
    )

    return RepositoryFixture(root, Repository.open(source))
