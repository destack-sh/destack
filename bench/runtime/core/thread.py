from bench.language import NodeReference


class ThreadHandle:
    """A handle to a Thread."""

    def __init__(self, thread_ptr: NodeReference):
        self.thread_ptr = thread_ptr
