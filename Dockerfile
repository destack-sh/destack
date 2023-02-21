# Use the official Python slim image as the base image
FROM python:3.11-slim

LABEL org.opencontainers.image.source=https://github.com/symbolx/bench
LABEL org.opencontainers.image.description="Bench API"

# Install postgresql-libs
RUN apt-get update && apt-get install -y libpq-dev libzbar-dev

ENV PYTHONUNBUFFERED 1
ENV PYTHONPATH "${PYTHONPATH}:/bench"

# Copy the requirements file into the container and install dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy the rest of the application code into the container
COPY bench/ bench/
COPY manage.py .
COPY pyproject.toml .

# Expose port 80
EXPOSE 80