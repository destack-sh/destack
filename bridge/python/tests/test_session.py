import re
import unittest

from destack import VERSION, ArtifactKey, Session, SourceFile, SourceSnapshot, version


class DestackSessionTest(unittest.TestCase):
    def test_version_is_available(self) -> None:
        self.assertNotEqual(VERSION, "")
        self.assertEqual(version(), VERSION)

    def test_open_source_session_reports_files(self) -> None:
        source = SourceSnapshot(
            [
                SourceFile.text("destack.json", '{"name":"@test/app"}'),
                SourceFile.text("src/index.ds", "export const value = 1;"),
            ]
        )

        session = Session.open_source("/virtual", source)
        files = session.files()

        self.assertEqual([file.path for file in files], ["destack.json", "src/index.ds"])

    def test_require_returns_artifact_version(self) -> None:
        source = SourceSnapshot(
            [
                SourceFile.text("destack.json", '{"name":"@test/app"}'),
                SourceFile.text("src/index.ds", "export const value = 1;"),
            ]
        )

        session = Session.open_source("/virtual", source)
        module = session.load_module("src/index.ds")
        version = session.require(session.revision(), ArtifactKey.dir_parsed(module.id))

        self.assertRegex(version.fingerprint, re.compile(r"^f[0-9a-f]{32}$"))


if __name__ == "__main__":
    unittest.main()
