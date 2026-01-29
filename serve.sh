#!/usr/bin/env bash

[ -d target ] || { echo "Not in workspace root; exiting" && exit 1; };

echo -n password > ./.password.txt

export BASE_PASSWORD_FILE=./.password.txt
export BASE_USERNAME=june
export PG_USER=junkie
export PG_DATABASE=barista
export PG_USER_PASSWORD_FILE=./.password.txt
export BACKEND_PORT=8741
export ASSET_DIR=/home/$USER/.local/state/blog_assets/

echo -e "Starting up \e[1mbackend\e[0m; all output from front and backend will be piped through here"

cargo leptos serve -p backend
