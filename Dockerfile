# Use the official Python slim image as the base image
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

# Copy the requirements file into the container and install dependencies
COPY requirements.txt .
COPY requirements-worker.txt .
# TODO @Cleanup: use separator worker image
RUN pip install --no-cache-dir -r requirements.txt -r requirements-worker.txt

# Copy the rest of the application code into the container
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