//! `sawbones-mcp`: the MCP surface v0 on stdio, serving the simulated bench.
//! Takes no arguments and reads no file. Nothing is written but MCP
//! messages on stdout; errors go to stderr.

use std::io::{self, BufReader};

fn main() {
    if std::env::args_os().len() > 1 {
        eprintln!("sawbones-mcp takes no arguments: it serves MCP on stdio");
        std::process::exit(2);
    }
    let bench = match sawbones_mcp::sim::SimBench::standard() {
        Ok(bench) => bench,
        Err(e) => {
            eprintln!("sawbones-mcp: cannot open the random source: {e}");
            std::process::exit(1);
        }
    };
    let mut server = sawbones_mcp::Server::new(bench);
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(e) = sawbones_mcp::serve(&mut server, BufReader::new(stdin.lock()), stdout.lock()) {
        eprintln!("sawbones-mcp: {e}");
        std::process::exit(1);
    }
}
