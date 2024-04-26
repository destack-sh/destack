# Define the common base image
FROM python:3.11-slim as base

LABEL org.opencontainers.image.source=https://github.com/symbolx/bench
LABEL org.opencontainers.image.description="Bench"

RUN apt-get update
# Install postgresql-libs
RUN apt-get install -y libpq-dev libzbar-dev
# Install ML libs
RUN apt-get install -y ffmpeg
# Install GCC and Fortran
RUN apt-get install -y gcc gfortran
RUN apt-get install -y pkg-config cmake libopenblas-dev liblapack-dev

ENV PYTHONUNBUFFERED 1
ENV PYTHONPATH "${PYTHONPATH}:/bench"


# --- System (Supervisor/Host) ---
FROM base as bench-system

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy all server files
COPY bench/ bench/
COPY manage.py .
COPY manageserver.py .
COPY pyproject.toml .
COPY version .

ARG GIT_COMMIT
ARG VERSION
ENV GIT_COMMIT $GIT_COMMIT
ENV VERSION $VERSION

EXPOSE 80

# --- Runtime ---
FROM base as bench-runtime

RUN apt-get install -y pandoc
RUN apt-get install -y tesseract-ocr libtesseract-dev libleptonica-dev tesseract-ocr-deu
RUN apt-get install -y libmagic1 libmagic-dev
RUN apt-get install -y poppler-utils
COPY requirements-runtime.txt .
RUN pip install --no-cache-dir -r requirements-runtime.txt

# Copy runtime-specific files (only!)
COPY bench/utils bench/utils
COPY bench/runtime bench/runtime
COPY bench/language bench/language
COPY bench/sql bench/sql
COPY bench/search bench/search
COPY bench/proto bench/proto
COPY manageruntime.py .
COPY pyproject.toml .
COPY version .

ARG GIT_COMMIT
ARG VERSION
ENV GIT_COMMIT $GIT_COMMIT
ENV VERSION $VERSION

EXPOSE 80
