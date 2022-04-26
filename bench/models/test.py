from bench.models import Function
from bench.models.execution import Execution, FunctionExecution


class Test(Function):
    class Meta:
        proxy = True


class TestExecution(FunctionExecution):
    class Meta:
        proxy = True


class TestSuite(Function):
    class Meta:
        proxy = True


class TestSuiteExecution(Execution):
    class Meta:
        proxy = True
