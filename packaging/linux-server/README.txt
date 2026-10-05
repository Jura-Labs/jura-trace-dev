Jura Trace for Linux servers
============================

The Jura Trace analysis server without the desktop app, for machines
nobody sits at. Everything is analysed on this machine.

  jura-trace-api   the server (the REST API on 127.0.0.1)
  jura             the command-line client
  jura-sidecar     the analysis engine the server starts and stops itself
  models/          the models the analysis engine needs

Neither jura-trace-api nor jura needs a desktop, GTK or a web view: only
the C library (glibc 2.35 or later; Ubuntu 22.04, Debian 12 or newer).
The analysis engine unpacks itself into the temporary directory each time
it starts, so allow about 1 GB there.

Keep the four together: jura-trace-api looks for jura-sidecar beside
itself, and for models/ beside that.

First run
---------

    ./jura-trace-api keys add --name my-server    # prints a key, once
    ./jura-trace-api                              # serves on 127.0.0.1:8300

In another terminal:

    export JURA_API_KEY=jt_...                    # the key printed above
    export JURA_NO_KEYRING=1                      # servers have no keyring
    ./jura verify photo.jpg --wait-ready

Without --db, the server keeps its database where the desktop app would:
$XDG_DATA_HOME/org.juralabs.trace, else ~/.local/share/org.juralabs.trace.

For a session that leaves nothing behind, use

    ./jura-trace-api --ephemeral

which prints a key for the session and deletes its database when it stops.

It listens on 127.0.0.1 only. To run it as a service, give systemd (or your
own supervisor) the full path; it stops cleanly on SIGTERM and stops the
analysis engine with it.

More: https://github.com/Jura-Labs/jura-trace/blob/main/docs/CLI_QUICKSTART.md
and docs/API_WRAPPER.md in the source. Licence: AGPL-3.0-or-later (LICENSE).
