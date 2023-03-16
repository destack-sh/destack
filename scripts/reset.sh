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
    read -p "Reset the DB [y/N]" -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted"
        exit 1
    fi
fi

# reset the db content
python manage.py flush --no-input

# run migrations
python manage.py migrate

# set up dev environment
python manage.py bootstrap && python manage.py setupdev