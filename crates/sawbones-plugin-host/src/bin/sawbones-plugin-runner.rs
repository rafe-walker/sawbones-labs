//! `sawbones-plugin-runner`: the sandboxed child that runs one plugin batch.
//! Spawned by `sawbones_plugin_host::PluginHost`; not meant to be run by hand.

fn main() {
    std::process::exit(sawbones_plugin_host::runner::main());
}
