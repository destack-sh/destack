import re
import unittest

from destack import VERSION, ArtifactKey, Edit, Session, Source, version


class DestackSessionTest(unittest.TestCase):
    def test_version_is_available(self) -> None:
        self.assertNotEqual(VERSION, "")
        self.assertEqual(version(), VERSION)

    def test_open_memory_session_reports_files(self) -> None:
        source = Source.memory(
            "/virtual",
            [
                Edit.set_text("destack.json", '{"name":"@test/app"}'),
                Edit.set_text("src/index.ds", "export const value = 1;"),
            ],
        )

        session = Session.open(source)
        files = session.files()

        self.assertEqual([file.path for file in files], ["destack.json", "src/index.ds"])

    def test_require_returns_artifact_version(self) -> None:
        source = Source.memory(
            "/virtual",
            [
                Edit.set_text("destack.json", '{"name":"@test/app"}'),
                Edit.set_text("src/index.ds", "export const value = 1;"),
            ],
        )

        session = Session.open(source)
        module = session.load_module("src/index.ds")
        version = session.require(session.revision(), ArtifactKey.dir_parsed(module.id))
        parsed = session.parse(session.revision(), module)

        self.assertRegex(version.fingerprint, re.compile(r"^f[0-9a-f]{32}$"))
        self.assertEqual(parsed.version.fingerprint, version.fingerprint)
        self.assertEqual(parsed.module.key, module.id.key)


if __name__ == "__main__":
    unittest.main()
