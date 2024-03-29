#!/usr/bin/env bash

set -eu

[ -d target ] || { echo "Not in workspace root; exiting" && exit 1; };

clean_up() {
	[ -z ${backend_pid+x} ] || kill "$backend_pid"
}

echo -e "Starting up \e[1mbackend\e[0m; all output from front and backend will be piped through here"

trap 'clean_up' INT ERR

# we build it in the foreground so that we can launch it in the background once we know it's all good
cargo build --bin backend --release
./target/release/backend &
backend_pid=$!

echo -e "\nGiving backend a few seconds to startup, then launching \e[1mtrunk\e[0m for \e[1mfrontend\e[0m\n"

cd frontend
trunk serve --proxy-backend='http://localhost:3000/api/'

