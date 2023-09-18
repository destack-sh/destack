# dump
python manage.py module dump flotothemoon/tests /tmp/bench/flotothemoon.tests

# copy to s3
aws s3 cp /tmp/bench/flotothemoon.tests s3://bench-internal/flotothemoon.tests --recursive
