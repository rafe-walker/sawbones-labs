//! End to end (#103): spawn the server binary, speak MCP to it over stdio,
//! and call every tool against the simulated bench.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    next: i64,
}

impl Client {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_sawbones-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn sawbones-mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Client {
            child,
            stdin: Some(stdin),
            stdout,
            next: 1,
        }
    }

    fn send(&mut self, message: &Value) {
        let mut line = serde_json::to_string(message).unwrap();
        line.push('\n');
        self.raw(line.as_bytes());
    }

    fn raw(&mut self, bytes: &[u8]) {
        let stdin = self.stdin.as_mut().unwrap();
        stdin.write_all(bytes).unwrap();
        stdin.flush().unwrap();
    }

    fn receive(&mut self) -> Value {
        let mut line = String::new();
        let n = self.stdout.read_line(&mut line).unwrap();
        assert!(n > 0, "the server closed stdout");
        assert!(line.ends_with('\n'));
        assert_eq!(line.matches('\n').count(), 1, "one message per line");
        let v: Value = serde_json::from_str(&line).unwrap();
        // The server never sends a request or notification of its own.
        assert!(v.get("method").is_none(), "server-initiated message: {v}");
        assert_eq!(v["jsonrpc"], "2.0");
        v
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        let reply = self.receive();
        assert_eq!(reply["id"], id);
        reply
    }

    fn call(&mut self, tool: &str, arguments: Value) -> (String, bool) {
        let reply = self.request(
            "tools/call",
            json!({ "name": tool, "arguments": arguments }),
        );
        let result = &reply["result"];
        assert!(result.is_object(), "{tool}: {reply}");
        let content = result["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        (
            content[0]["text"].as_str().unwrap().to_owned(),
            result["isError"].as_bool().unwrap(),
        )
    }

    fn initialize(&mut self) -> Value {
        let reply = self.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "e2e", "version": "0" },
            }),
        );
        self.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        reply
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn initialize_list_and_call_every_tool_over_stdio() {
    let mut c = Client::spawn();
    let init = c.initialize();
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(
        init["result"]["capabilities"],
        json!({ "tools": { "listChanged": false } })
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "sawbones-mcp");

    let list = c.request("tools/list", json!({}));
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "bench_status",
            "inventory",
            "describe",
            "read",
            "session_query"
        ]
    );

    let (status, err) = c.call("bench_status", json!({}));
    assert!(!err);
    assert_eq!(status.lines().count(), 1, "one status line: {status}");
    assert!(status.starts_with("bench (simulated)"));
    assert!(status.contains("3 enrolled, 3 available"), "{status}");
    assert!(status.contains("rail A off, rail B off, fault none"));
    assert!(status.contains("latch: tripped by supervisor start"));

    let (inventory, err) = c.call("inventory", json!({}));
    assert!(!err);
    assert!(
        inventory.contains("module 1 'dc-meter' on hub port 0: available"),
        "{inventory}"
    );
    assert!(inventory.contains("module 2 'comms-pod' on hub port 1: available"));
    assert!(inventory.contains("module 3 'interlock-controller' on hub port 2: available"));
    assert!(!inventory.contains("UNTRUSTED_DATA"));

    let (describe, err) = c.call("describe", json!({ "module": 1 }));
    assert!(!err, "{describe}");
    assert!(describe.contains("hash matches the digest pinned at enrollment; handle mf-1"));
    assert_eq!(describe.matches(sawbones_reading::FENCE_OPEN).count(), 2);
    assert_eq!(describe.matches(sawbones_reading::FENCE_CLOSE).count(), 2);
    assert!(
        describe.contains("latch: tripped by device input in turn 3"),
        "{describe}"
    );

    let (read, err) = c.call("read", json!({ "module": 1, "key": 256, "samples": 8 }));
    assert!(!err, "{read}");
    assert!(read.contains("8 asked, 8 answered with a value"), "{read}");
    assert!(read.contains("handle rd-2"));
    assert!(read.contains("values the module claims (device-reported, not measured)"));
    // A summary, never the raw samples: the statistics line has four values.
    let claim = read.lines().find(|l| l.starts_with("values")).unwrap();
    let (_, stats) = claim.split_once("): ").unwrap();
    assert!(stats.starts_with("last "), "{claim}");
    assert_eq!(stats.matches(", ").count(), 3, "{claim}");

    let (listing, err) = c.call("session_query", json!({}));
    assert!(!err);
    assert!(listing.contains("rd-2 read: module 1 key 0x0100, 8 sample(s) of 8 asked"));
    assert!(listing.contains("mf-1 manifest: module 1, 2 page(s), digest matches"));

    let (window, err) = c.call(
        "session_query",
        json!({ "handle": "rd-2", "from": 2, "to": 6 }),
    );
    assert!(!err, "{window}");
    assert!(window.contains("samples 2..6 of 8"));

    let (manifest, err) = c.call("session_query", json!({ "handle": "mf-1" }));
    assert!(!err);
    assert!(manifest.contains("call describe for the text"));
    assert!(!manifest.contains("UNTRUSTED_DATA"));

    let (status, _) = c.call("bench_status", json!({}));
    assert!(status.contains("2 stored handle(s)"), "{status}");
    assert!(status.contains("latch: tripped by device input"));

    assert_eq!(c.request("ping", json!({}))["result"], json!({}));
}

#[test]
fn only_read_class_tools_exist_and_the_invoke_names_are_not_callable() {
    let mut c = Client::spawn();
    c.initialize();
    let list = c.request("tools/list", json!({}));
    for tool in list["result"]["tools"].as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        let def = sawbones_mcp::TOOLS
            .iter()
            .find(|t| t.name == name)
            .expect("listed tool is registered");
        assert_eq!(def.class, sawbones_mcp::ToolClass::Read, "{name}");
        assert_eq!(tool["annotations"]["readOnlyHint"], true, "{name}");
        assert_eq!(tool["annotations"]["destructiveHint"], false, "{name}");
        assert!(!name.starts_with("invoke"), "{name}");
    }
    for name in sawbones_mcp::RESERVED_INVOKE_NAMES.iter().copied().chain([
        "invoke", "write", "arm", "lease", "actuate", "Read", "read ", "",
    ]) {
        let reply = c.request("tools/call", json!({ "name": name, "arguments": {} }));
        assert_eq!(reply["error"]["code"], -32602, "{name}: {reply}");
        assert_eq!(reply["error"]["message"], "unknown tool");
    }
    // The server still answers afterwards.
    let (_, err) = c.call("bench_status", json!({}));
    assert!(!err);
}

#[test]
fn protocol_edges() {
    let mut c = Client::spawn();
    // Tools before initialize are refused; ping is not.
    let early = c.request("tools/list", json!({}));
    assert_eq!(early["error"]["code"], -32002);
    assert_eq!(c.request("ping", json!({}))["result"], json!({}));
    c.initialize();
    // Methods v0 does not have, including every server-to-client one.
    for method in [
        "sampling/createMessage",
        "elicitation/create",
        "roots/list",
        "resources/list",
        "prompts/list",
        "logging/setLevel",
        "completion/complete",
    ] {
        let reply = c.request(method, json!({}));
        assert_eq!(reply["error"]["code"], -32601, "{method}");
    }
    // A notification and a stray response get no reply; the next request
    // is still answered in order.
    c.send(&json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": {} }));
    c.send(&json!({ "jsonrpc": "2.0", "id": 99, "result": {} }));
    assert_eq!(c.request("ping", json!({}))["result"], json!({}));
    // Garbage, a batch and an over-long line each get an error.
    c.raw(b"{not json\n");
    assert_eq!(c.receive()["error"]["code"], -32700);
    c.raw(b"[{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}]\n");
    assert_eq!(c.receive()["error"]["code"], -32600);
    let long = vec![b'x'; sawbones_mcp::MAX_MESSAGE_BYTES + 1];
    c.raw(&long);
    c.raw(b"\n");
    assert_eq!(c.receive()["error"]["code"], -32600);
    // Arguments that are not the tool's are refused as a tool error.
    let (text, err) = c.call(
        "read",
        json!({ "module": 1, "key": 256, "hazard_class": 0 }),
    );
    assert!(err);
    assert!(text.starts_with("invalid arguments"), "{text}");
    // A module nobody enrolled.
    let (text, err) = c.call("describe", json!({ "module": 9 }));
    assert!(err);
    assert!(text.starts_with("module 9 is not enrolled"), "{text}");
    // A handle that is not one.
    let (text, err) = c.call("session_query", json!({ "handle": "rd-0" }));
    assert!(err);
    assert!(text.starts_with("not a handle"), "{text}");
    // An unknown protocol version gets the newest.
    let reply = c.request("initialize", json!({ "protocolVersion": "1999-01-01" }));
    assert_eq!(
        reply["result"]["protocolVersion"],
        sawbones_mcp::PROTOCOL_VERSIONS[0]
    );
    // End of input ends the server cleanly.
    drop(c.stdin.take());
    let status = c.child.wait().unwrap();
    assert!(status.success(), "{status}");
}
