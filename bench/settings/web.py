# Application definition
import os
from datetime import timedelta

from bench.settings.base import DEBUG, TEST
from bench.settings.utils import get_from_env, str_to_bool

INSTALLED_APPS = [
    "django.contrib.admin",
    "django.contrib.auth",
    "django.contrib.contenttypes",
    "django.contrib.sessions",
    "django.contrib.messages",
    "django.contrib.staticfiles",
    "django.contrib.postgres",
    "strawberry.django",
    "rest_framework",
    "loginas",
    "corsheaders",
    "social_django",
    "django_filters",
    "django_prometheus",
    "drf_spectacular",
    "bench.apps.BenchConfig",
]

MIDDLEWARE = [
    "django_prometheus.middleware.PrometheusBeforeMiddleware",
    "django_structlog.middlewares.RequestMiddleware",
    "corsheaders.middleware.CorsMiddleware",
    "django.middleware.security.SecurityMiddleware",
    "django.contrib.sessions.middleware.SessionMiddleware",
    "django.middleware.common.CommonMiddleware",
    "django.middleware.csrf.CsrfViewMiddleware",
    "django.contrib.auth.middleware.AuthenticationMiddleware",
    "django.contrib.messages.middleware.MessageMiddleware",
    "django.middleware.clickjacking.XFrameOptionsMiddleware",
    "django_prometheus.middleware.PrometheusAfterMiddleware",
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

# Auth

AUTH_USER_MODEL = "bench.User"

SOCIAL_AUTH_JSONFIELD_ENABLED = True
SOCIAL_AUTH_USER_MODEL = "bench.User"
SOCIAL_AUTH_REDIRECT_IS_HTTPS = get_from_env(
    "SOCIAL_AUTH_REDIRECT_IS_HTTPS", not DEBUG, type_cast=str_to_bool
)
SOCIAL_AUTH_URL_NAMESPACE = "social"

AUTHENTICATION_BACKENDS: list[str] = [
    "axes.backends.AxesBackend",
    "social_core.backends.github.GithubOAuth2",
    "social_core.backends.gitlab.GitLabOAuth2",
    "django.contrib.auth.backends.ModelBackend",
]

SOCIAL_AUTH_PIPELINE = (
    "social_core.pipeline.social_auth.social_details",
    "social_core.pipeline.social_auth.social_uid",
    "social_core.pipeline.social_auth.auth_allowed",
    "social_core.pipeline.social_auth.social_user",
    "social_core.pipeline.social_auth.associate_by_email",
    "bench.api.signup.social_create_user",
    "social_core.pipeline.social_auth.associate_user",
    "social_core.pipeline.social_auth.load_extra_data",
    "social_core.pipeline.user.user_details",
)

SOCIAL_AUTH_STRATEGY = "social_django.strategy.DjangoStrategy"
SOCIAL_AUTH_STORAGE = "social_django.models.DjangoStorage"
SOCIAL_AUTH_FIELDS_STORED_IN_SESSION = [
    "invite_id",
    "user_name",
    "email_opt_in",
    "organization_name",
]

# Auth - social GitHub
SOCIAL_AUTH_GITHUB_SCOPE = ["user:email"]
SOCIAL_AUTH_GITHUB_KEY = os.environ.get("SOCIAL_AUTH_GITHUB_KEY")
SOCIAL_AUTH_GITHUB_SECRET = os.environ.get("SOCIAL_AUTH_GITHUB_SECRET")

# Auth - via GitLab
SOCIAL_AUTH_GITLAB_SCOPE = ["read_user"]
SOCIAL_AUTH_GITLAB_KEY = os.environ.get("SOCIAL_AUTH_GITLAB_KEY")
SOCIAL_AUTH_GITLAB_SECRET = os.environ.get("SOCIAL_AUTH_GITLAB_SECRET")
SOCIAL_AUTH_GITLAB_API_URL = os.environ.get("SOCIAL_AUTH_GITLAB_API_URL", "https://gitlab.com")

# Axes

AXES_ENABLED = get_from_env("AXES_ENABLED", not TEST, type_cast=str_to_bool)
AXES_HANDLER = "axes.handlers.cache.AxesCacheHandler"
AXES_FAILURE_LIMIT = get_from_env("AXES_FAILURE_LIMIT", 30, type_cast=int)
AXES_COOLOFF_TIME = timedelta(minutes=10)


# Password validation
# https://docs.djangoproject.com/en/4.0/ref/settings/#auth-password-validators

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
# https://docs.djangoproject.com/en/4.0/topics/i18n/

LANGUAGE_CODE = "en-us"

TIME_ZONE = "UTC"

USE_I18N = True

USE_L10N = True

USE_TZ = True


# Static files (CSS, JavaScript, Images)
# https://docs.djangoproject.com/en/4.0/howto/static-files/

STATIC_URL = "/static/"

# Extra misc settings


CSRF_COOKIE_NAME = "bench_csrftoken"

REST_FRAMEWORK = {
    "DEFAULT_SCHEMA_CLASS": "drf_spectacular.openapi.AutoSchema",
    "EXCEPTION_HANDLER": "exceptions_hog.exception_handler",
}

EXCEPTIONS_HOG = {
    "EXCEPTION_REPORTING": "exceptions_hog.handler.exception_reporter",
    "ENABLE_IN_DEBUG": False,
    "NESTED_KEY_SEPARATOR": "__",
    "SUPPORT_MULTIPLE_EXCEPTIONS": True,
}
