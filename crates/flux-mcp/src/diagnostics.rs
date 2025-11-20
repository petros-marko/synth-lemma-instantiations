#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticTarget {
    pub name: String,
    pub kind: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticSpan {
    pub file_name: String,
    pub line_start: i64,
    pub column_start: i64,
    pub line_end: i64,
    pub column_end: i64,
    #[serde(default)]
    pub is_primary: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticMessage {
    pub level: String,
    pub message: String,
    pub code: Option<String>,
    pub rendered: Option<String>,
    pub spans: Vec<DiagnosticSpan>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Diagnostic {
    pub message: DiagnosticMessage,
    pub package_id: Option<String>,
    pub target: Option<DiagnosticTarget>,
}

fn parse_spans(spans: &serde_json::Value) -> Option<Vec<DiagnosticSpan>> {
    let mut res = Vec::new();
    for span in spans.as_array()? {
        let file_name = span.get("file_name")?.as_str()?.to_string();
        let line_start = span.get("line_start")?.as_i64().unwrap_or(0);
        let column_start = span.get("column_start")?.as_i64().unwrap_or(0);
        let line_end = span.get("line_end")?.as_i64().unwrap_or(0);
        let column_end = span.get("column_end")?.as_i64().unwrap_or(0);
        let is_primary = span.get("is_primary")?.as_bool().unwrap_or(true);
        res.push(DiagnosticSpan {
            file_name,
            line_start,
            column_start,
            line_end,
            column_end,
            is_primary,
        })
    }
    Some(res)
}

pub(crate) fn parse_message(message: &serde_json::Value) -> Option<DiagnosticMessage> {
    let level = message.get("level")?.as_str()?.to_string();
    let code = message
        .get("code")
        .and_then(|code| code.as_str().map(|code| code.to_string()));
    let rendered = message
        .get("rendered")
        .and_then(|rendered| rendered.as_str().map(|rendered| rendered.to_string()));
    let spans = message.get("spans").and_then(parse_spans).unwrap_or(vec![]);
    let message = message.get("message")?.as_str()?.to_string();
    Some(DiagnosticMessage { level, message, code, rendered, spans })
}

pub(crate) fn parse_target(target: &serde_json::Value) -> Option<DiagnosticTarget> {
    let name = target.get("name")?.as_str()?.to_string();
    let kind = target.get("kind");
    if let Some(kind) = kind {
        let mut kinds = Vec::new();
        for k in kind.as_array()? {
            kinds.push(k.as_str()?.to_string())
        }
        Some(DiagnosticTarget { name, kind: Some(kinds) })
    } else {
        Some(DiagnosticTarget { name, kind: None })
    }
}

pub(crate) fn retain_only_syntax_errors(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let flux_error_markers: &[&str] = &[
        "error jumping to join point",
        "assignment might be unsafe",
        "call to function that may panic",
        "refinement type error",
        "possible division by zero",
        "possible reminder with a divisor of zero",
        "assertion might fail",
        "parameter inference error at function call",
        "type invariant may not hold (when place is folded)",
        "cannot prove this code safe",
        "arithmetic operation may overflow",
        "arithmetic operation may underflow",
        "unsupported type in function call",
        "invariant cannot be proven",
        "associated refinement"
    ];
    diagnostics
        .into_iter()
        .filter(|diag| {
            diag.message.level.as_str() == "error"
                && !flux_error_markers.iter().any(|marker| diag.message.message.contains(marker))
        })
        .collect()
}
