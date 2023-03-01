import posthog

from bench.settings import DEBUG, TEST
from bench.utils.analytics import init_sentry

# Sentry

if not (TEST or DEBUG):
    init_sentry()

# Posthog
posthog.project_api_key = "phc_d8mi3OMdtKSVA8kzHbBoKtYU3ZsMQakAiLpuOn3W9ma"
posthog.host = "https://eu.posthog.com"

if TEST:
    posthog.disabled = True
