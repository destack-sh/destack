from bench.test.simulation.spec import SimulationSpec


class Simulation:
    def __init__(self, spec: SimulationSpec):
        self.spec = spec

    async def run(self):
        raise NotImplementedError
