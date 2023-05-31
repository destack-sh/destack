# Define the common base image
FROM python:3.11-slim as base

LABEL org.opencontainers.image.source=https://github.com/symbolx/bench
LABEL org.opencontainers.image.description="Bench API"

RUN apt-get update
# Install postgresql-libs
RUN apt-get install -y libpq-dev libzbar-dev
# Install ML libs
RUN apt-get install -y ffmpeg

ENV PYTHONUNBUFFERED 1
ENV PYTHONPATH "${PYTHONPATH}:/bench"

# Copy the requirements file into the container
COPY requirements.txt .
COPY requirements-worker.txt .

# Install dependencies from requirements.txt
# TODO @Cleanup @Architecture: use separate requirements for worker
RUN pip install --no-cache-dir -r requirements.txt -r requirements-worker.txt

# Define the API image
FROM base as bench-api

# Copy the application code into the container
COPY bench/ bench/
COPY manage.py .
COPY pyproject.toml .
COPY version .

ARG GIT_COMMIT
ARG VERSION
ENV GIT_COMMIT $GIT_COMMIT
ENV VERSION $VERSION

# Expose port 80
EXPOSE 80

# Define the worker image
FROM base as bench-worker

# Copy the additional requirements file for the worker and install dependencies
COPY requirements-worker.txt .
RUN pip install --no-cache-dir -r requirements-worker.txt

# Copy the specific directories and files for the worker
COPY bench/utils/ bench/utils/
COPY bench/runtime/ bench/runtime/
COPY bench/language/ bench/language/
COPY bench/msg/ bench/msg/
COPY bench/runworker.py bench/runworker.py
COPY pyproject.toml pyproject.toml
COPY version .