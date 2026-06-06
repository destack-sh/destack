import unittest

from destack import create_client
from destack_py import create_client as create_alias_client


class DestackPyAliasTest(unittest.TestCase):
    def test_alias_exports_canonical_client(self) -> None:
        client = create_client()
        alias_client = create_alias_client()

        self.assertEqual(client.backend, "python")
        self.assertEqual(alias_client.backend, "python")
        self.assertEqual(alias_client.version(), client.version())


if __name__ == "__main__":
    unittest.main()
