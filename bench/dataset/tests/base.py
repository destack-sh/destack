from django.test import TestCase

from bench.models import DatasetVersion


class TestDatasetHandler(TestCase):
    def setUp(self):
        DatasetVersion.objects.create()
