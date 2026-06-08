import os
import unittest

from destack import BACKEND, VERSION, DestackClient, create_client


class DestackClientTest(unittest.TestCase):
    def test_create_client_returns_python_backend(self) -> None:
        client = create_client()

        self.assertIsInstance(client, DestackClient)
        self.assertEqual(client.backend, BACKEND)
        self.assertEqual(client.backend, "python")

    def test_version_is_available(self) -> None:
        client = create_client()

        self.assertNotEqual(client.version(), "")
        self.assertNotEqual(VERSION, "")

    def test_explicit_capi_library_reports_abi_version(self) -> None:
        if "DESTACK_CAPI_LIB" not in os.environ:
            self.skipTest("DESTACK_CAPI_LIB is not configured")

        client = create_client()

        self.assertGreater(client.capi_abi_version(), 0)


if __name__ == "__main__":
    unittest.main()
