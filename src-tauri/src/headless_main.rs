// SPDX-License-Identifier: AGPL-3.0-or-later

//! `jura-trace-api`: the Jura Trace REST API without the desktop window.
//! Everything is in `jura_trace_lib::headless`.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(jura_trace_lib::headless::main_with_args(&args));
}
