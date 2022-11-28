#!/bin/bash

# ensure we're in the right directory (the root of the 'bench' project)
if [ ! -f "manage.py" ]; then
    echo "run this script from the root of the 'bench' project"
    exit 1
fi

# set the right venv
source venv/bin/activate

# confirm unless --force is passed
if [ "$1" != "--force" ]; then
    read -p "Reset the DB (and all migrations if --hard) [y/N]" -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted"
        exit 1
    fi
fi

# reset hard if --hard is passed
if [ "$1" == "--hard" ]; then
    # shut down docker and wipe volumes
    docker compose -f docker-compose.dev.yml down --volumes

    # create new docker env
    docker compose -f docker-compose.dev.yml up -d

    # wait a bit for the db to start
    sleep 3

    # remove all migrations
    rm -rf bench/migrations

    # create new migrations
    python manage.py makemigrations bench
fi

# run migrations
python manage.py migrate

# set up dev environment
python manage.py setupdev