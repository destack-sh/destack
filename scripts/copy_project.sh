#!/bin/bash
#
# Copies a project from one DB environment to the other.
# 1. Dump it with env of source env to a local file
# 2. Load it with env of target env from the local file
#

#!/bin/bash

set -e

# Parse command line arguments
while getopts "s:t:p:" opt; do
case $opt in
s)
source_env="$OPTARG"
;;
t)
target_env="$OPTARG"
;;
p)
project_path="$OPTARG"
;;
?)
echo "Invalid option: -$OPTARG" >&2
exit 1
;;
:)
echo "Option -$OPTARG requires an argument." >&2
exit 1
;;
esac
done

# Check if required arguments are set
if [[ -z $source_env || -z $target_env || -z $project_path ]]; then
echo "Usage: $0 -s <source_env> -t <target_env> -p <project_path>"
exit 1
fi

echo "copying $project_path from $source_env to $target_env"

# Clean project path (normalize non-alphanumeric characters)
tmp_name=$(echo "$project_path" | tr -cd '[:alnum:]_')

# Dump the project from source DB environment
echo "dumping $project_path from $source_env to tmp_$tmp_name.bench"
LOCAL_ENV="$source_env" python manage.py dump "$project_path" > "tmp_$tmp_name.bench"

# Load the project to target DB environment
echo "loading $project_path from tmp_$tmp_name.bench to $target_env"
LOCAL_ENV="$target_env" python manage.py load "$project_path" "tmp_$tmp_name.bench" > /dev/null 2>&1

# Remove temporary dump file
rm "tmp_$tmp_name.bench"
echo "done"