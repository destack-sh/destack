#!/bin/bash

# check we're in the right directory (the root of the 'bench' project)
if [ ! -f "manage.py" ]; then
    echo "run this script from the root of the 'bench' project"
    exit 1
fi

# update the generated graphql schema
python manage.py exportschema > schema.gen.graphql

# update the generated graphql types
yarn graphql-codegen
