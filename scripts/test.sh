# exit on error
set -e

# try to upsert libs
echo "test generate default libraries"
python manage.py libs upsert all

# run module tests
echo "test module"
python manage.py module test flotothemoon/tests _shared.run_all_tests
