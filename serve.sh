#!/usr/bin/env bash

[ -d target ] || { echo "Not in workspace root; exiting" && exit 1; };

clean_up() {
	[ -z ${backend_pid+x} ] || kill "$backend_pid" 2>/dev/null
	[ -z ${trunk_pid+x} ] || kill "$trunk_pid" 2>/dev/null
}

echo -e "Starting up \e[1mbackend\e[0m; all output from front and backend will be piped through here"

trap 'clean_up' INT ERR

# we build it in the foreground so that we can launch it in the background once we know it's all good
cargo leptos build
cargo leptos watch &
backend_pid=$!

echo -e "\nGiving backend a few seconds to startup, then launching \e[1mtrunk\e[0m for \e[1mfrontend\e[0m\n"

cd frontend
trunk serve --proxy-backend='http://localhost:3000/api/' &
trunk_pid=$!

if ! ps -p "$backend_pid" >/dev/null 2>&1
then
	echo -e "\e[1;35mBackend died :(\e[0m"
	clean_up
	exit
fi

wait $backend_pid
wait $trunk_pid
