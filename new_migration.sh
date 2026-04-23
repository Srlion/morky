#!/bin/bash

name=$1
if [ -z "$name" ]; then
    echo "Usage: ./new_migration.sh <name>"
    exit 1
fi

dir="migrations"
mkdir -p "$dir"

timestamp=$(date +%Y%m%d%H%M%S)
file="$dir/${timestamp}_${name}.sql"

echo "-- Migration: $name" > "$file"
echo "Created: $file"
