import posthog

from bench.settings import DEBUG, TEST
from bench.utils.analytics import init_sentry
from bench.utils.utils import SOME_TYPE_CHECKING

# Sentry

if not (TEST or DEBUG or SOME_TYPE_CHECKING):
    init_sentry()

# Posthog
posthog.bench_api_key = "phc_d8mi3OMdtKSVA8kzHbBoKtYU3ZsMQakAiLpuOn3W9ma"
posthog.host = "https://eu.posthog.com"

if TEST or DEBUG:
    posthog.disabled = True
