from bench.language.core.const import RunType
from bench.language.runtime.run import RunOptions

ATTEMPT_ONCE = RunOptions(max_attempts=1)
ATTEMPT_THRICE = RunOptions(max_attempts=3)
ATTEMPT_FOREVER = RunOptions(max_attempts=None)
BASE_RUN_OPTIONS_BY_KIND = {
    RunType.CODE: ATTEMPT_ONCE,
    RunType.ACTION: ATTEMPT_THRICE,
    RunType.ACTION: ATTEMPT_ONCE,
    RunType.PIPE: ATTEMPT_ONCE,
    RunType.FLOW: ATTEMPT_ONCE,
}
