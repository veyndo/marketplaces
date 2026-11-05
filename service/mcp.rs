//! Stateless JSON MCP adapter for the same trusted-gateway actions; schemas come from the installed manifest.
use super::*;
const MANIFEST: &str = include_str!("../manifest.json");

pub(super) async fn handle(
    State(a): State<App>,
    h: HeaderMap,
    Json(v): Json<Value>,
) -> Result<Json<Value>> {
    let tenant = a.tenant(&h)?.to_owned();
    let id = v.get("id").cloned().unwrap_or(Value::Null);
    let fail = |code, message: &str| {
        Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}}))
    };
    if v["jsonrpc"] != "2.0" || !(id.is_string() || id.is_i64() || id.is_u64()) {
        return Ok(fail(
            -32600,
            "Request requires JSON-RPC 2.0 and a string or integer ID",
        ));
    }
    let manifest: Value = serde_json::from_str(MANIFEST).expect("bundled manifest");
    let actions: Vec<&Value> = manifest["actions"]
        .as_array()
        .expect("actions")
        .iter()
        .filter(|action| action["handler"] == "service" && action["mcp"] == true)
        .collect();
    let result = match v["method"].as_str() {
        Some("initialize") => {
            json!({"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"veyndo-marketplaces","version":env!("CARGO_PKG_VERSION")}})
        }
        Some("tools/list") => json!({"tools":actions.iter().map(|action| json!({
            "name":action["name"],"description":action["description"],"inputSchema":action["inputSchema"],
            "annotations":{"readOnlyHint":action["readOnly"],"idempotentHint":false,"openWorldHint":true}
        })).collect::<Vec<_>>()}),
        Some("tools/call") => {
            let name = v["params"]["name"].as_str().unwrap_or("");
            if !actions.iter().any(|action| action["name"] == name) {
                return Ok(fail(-32602, "Unknown marketplace tool"));
            }
            let args = &v["params"]["arguments"];
            if !args.is_object() {
                return Ok(fail(-32602, "Arguments must be an object"));
            }
            match runtime::dispatch(&a, &tenant, name, args).await {
                Ok(value) => {
                    json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":value,"isError":false})
                }
                Err((code, Json(value))) => {
                    json!({"content":[{"type":"text","text":value.to_string()}],"isError":true,"_meta":{"httpStatus":code.as_u16()}})
                }
            }
        }
        _ => return Ok(fail(-32601, "Method not found")),
    };
    Ok(Json(json!({"jsonrpc":"2.0","id":id,"result":result})))
}
