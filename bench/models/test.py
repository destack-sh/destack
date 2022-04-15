from django.db import models

from bench.models.utils import MAX_NAME_LENGTH, UUIDModel


class Test(UUIDModel):
    name = models.CharField(max_length=MAX_NAME_LENGTH)


class TestRun(UUIDModel):
    test = models.ForeignKey("Test", on_delete=models.CASCADE)
    test_suite_run = models.ForeignKey("TestRun", on_delete=models.CASCADE, null=True)


class TestSuite(UUIDModel):
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    tests = models.ManyToManyField("Test")


class TestSuiteRun(UUIDModel):
    test_suite = models.ForeignKey("TestSuite", on_delete=models.CASCADE)
