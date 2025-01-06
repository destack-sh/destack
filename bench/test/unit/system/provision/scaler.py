from bench.language import ScalerStrategy, ScalerType
from bench.test.unit.conftest import RuntimeHandle


async def test_create_scaler(hosted_runtime: RuntimeHandle):
    """Creates a scaler."""
    _ = hosted_runtime.bench.scalers.create(
        type=ScalerType.MACHINE,
        strategy=ScalerStrategy.AUTO,
        name="Machine Scaler",
        min_count=1,
        target_count=1,
        max_count=4,
    )
    await hosted_runtime.commit()
