# exit on error
set -e

export DEBUG=1

# try to upsert libs
echo "test generate default libraries"
python manage.py libs upsert all

# run module tests
echo "test module"
venv-worker/bin/python manageworker.py run flotothemoon/tests _shared.run_all_tests

# run example entry points
# TODO @Test: run Bench examples
# venv-worker/bin/python manage.py module run symbolx/examples ???