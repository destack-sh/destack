#!/bin/bash
#
# Copies a project from one DB environment to the other.
# 1. Dump it with env of source env to a local file
# 2. Load it with env of target env from the local file
#

#!/bin/bash

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

# Clean project path (normalize non-alphanumeric characters)
project_path=$(echo "$project_path" | tr -cd '[:alnum:]/')

# Dump the project from source DB environment
LOCAL_ENV="$source_env" python manage.py dump "$project_path" > "tmp_$project_path.bench"

# Load the project to target DB environment
LOCAL_ENV="$target_env" python manage.py dump "$project_path" "-" < "tmp_$project_path.bench"

# Remove temporary dump file
rm "tmp_$project_path.bench"