from random import Random
from typing import cast

from attr import dataclass

Samplable = int | float


@dataclass
class SampledValue[T: Samplable]:
    min: T
    mean: T
    max: T


SampledInt = SampledValue[int]
SampledFloat = SampledValue[float]


def to_value[T: Samplable](random: Random, value: T | SampledValue[T]) -> T:
    if isinstance(value, SampledValue):
        sampled = random.triangular(value.min, value.max, value.mean)
        if isinstance(value.min, int):
            return cast(T, round(sampled))
        else:
            return cast(T, sampled)
    else:
        return value


def to_value_maybe[T: Samplable](random: Random, value: T | SampledValue[T] | None) -> T | None:
    if value is None:
        return None
    else:
        return to_value(random, value)
