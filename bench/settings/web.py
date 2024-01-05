# Application definition

from bench.utils.utils import SOME_TYPE_CHECKING, get_from_env, str_to_bool

INSTALLED_APPS = [
    "daphne",
    "django.contrib.admin",
    "django.contrib.auth",
    "django.contrib.contenttypes",
    "django.contrib.sessions",
    "django.contrib.messages",
    "django.contrib.staticfiles",
    "django.contrib.postgres",
    "pgcrypto",
    "bench.apps.BenchConfig",
]

# TODO @Cleanup: Daphne doesn't use MIDDLEWARE, so keeping this just for REST is confusing/inconsistent
# Do not touch these middlewares unless you're really sure you're not adding any sync middleware.
# This must run async end to end to ensure our REST endpoints run in the main thread (not in an executor),
# which is essential because event loops don't like threads and NATS particularly gets very confused..
MIDDLEWARE = [
    "django.middleware.security.SecurityMiddleware",
    "django.contrib.sessions.middleware.SessionMiddleware",
    "django.middleware.common.CommonMiddleware",
    "django.contrib.auth.middleware.AuthenticationMiddleware",
    "django.contrib.messages.middleware.MessageMiddleware",
    "django.middleware.clickjacking.XFrameOptionsMiddleware",
]

ROOT_URLCONF = "bench.urls"
APPEND_SLASH = False

TEMPLATES = [
    {
        "BACKEND": "django.template.backends.django.DjangoTemplates",
        "DIRS": [],
        "APP_DIRS": True,
        "OPTIONS": {
            "context_processors": [
                "django.template.context_processors.debug",
                "django.template.context_processors.request",
                "django.contrib.auth.context_processors.auth",
                "django.contrib.messages.context_processors.messages",
            ],
        },
    },
]

WSGI_APPLICATION = "bench.wsgi.application"
ASGI_APPLICATION = "bench.asgi.application"

# Where are we?
WEBAPP_URL = get_from_env("WEBAPP_URL", type_cast=str, optional=SOME_TYPE_CHECKING)

# Emails
LOOPS_API_KEY = get_from_env("LOOPS_API_KEY", type_cast=str, optional=True)
LOOPS_USER_TRANSACTIONAL_ID = get_from_env(
    "LOOPS_USER_TRANSACTIONAL_ID", type_cast=str, optional=True
)

# Strawberry

STRAWBERRY_DJANGO = {
    "MUTATIONS_DEFAULT_HANDLE_ERRORS": True,
    "MUTATIONS_DEFAULT_ARGUMENT_NAME": "input",
}
BROTLI_COMPRESSION_ENABLED = get_from_env("BROTLI_COMPRESSION_ENABLED", True, type_cast=str_to_bool)
BROTLI_QUALITY_LEVEL = get_from_env("BROTLI_QUALITY_LEVEL", 4, type_cast=int)

# Password validation
# https://docs.djangobench.com/en/4.0/ref/settings/#auth-password-validators

AUTH_PASSWORD_VALIDATORS = [
    {
        "NAME": "django.contrib.auth.password_validation.MinimumLengthValidator",
    },
    {
        "NAME": "django.contrib.auth.password_validation.CommonPasswordValidator",
    },
    {
        "NAME": "django.contrib.auth.password_validation.NumericPasswordValidator",
    },
]

# Internationalization
# https://docs.djangobench.com/en/4.0/topics/i18n/

LANGUAGE_CODE = "en-us"

TIME_ZONE = "UTC"

USE_I18N = True

USE_L10N = True

USE_TZ = True

# Static files (CSS, JavaScript, Images)
# https://docs.djangobench.com/en/4.0/howto/static-files/

STATIC_URL = "/static/"

# Extra misc settings


CSRF_COOKIE_NAME = "bench_csrftoken"

EXCEPTIONS_HOG = {
    "EXCEPTION_REPORTING": "exceptions_hog.handler.exception_reporter",
    "ENABLE_IN_DEBUG": False,
    "NESTED_KEY_SEPARATOR": "__",
    "SUPPORT_MULTIPLE_EXCEPTIONS": True,
}
