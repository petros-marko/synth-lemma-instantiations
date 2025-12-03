use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
};

use rmcp::schemars::{self, JsonSchema};

use crate::diagnostics::{Diagnostic, DiagnosticTarget, parse_message, parse_target};

pub struct FluxRunner {}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct VerifyRepositoryArgs {
    pub repo_path: String,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct VerifyPackageArgs {
    pub repo_path: String,
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerificationReport {
    pub success: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct GetLemmaArgs {
    pub repo_path: String
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lemma {
    pub name: String,
    pub file_name: String,
    pub start_line: i64,
    pub start_col: i64,
    pub end_line: i64,
    pub end_col: i64,
}

impl FluxRunner {
    pub fn new() -> Self {
        Self {}
    }

    fn flux_command(
        repo_root: &str,
        packages: Option<&[&str]>,
        flux_flags: Option<&[&str]>,
    ) -> Command {
        let mut cmd = Command::new("cargo");
        if let Some(flux_flags) = flux_flags {
            cmd.env("FLUXFLAGS", flux_flags.join(" "));
        }
        let mut args = vec!["flux".to_string()];
        if let Some(packages) = packages {
            for package in packages {
                args.push("-p".to_string());
                args.push(package.to_string());
            }
        }
        args.push("--message-format=json".to_string());
        cmd.current_dir(Path::new(repo_root));
        cmd.args(&args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd
    }

    fn parse_flux_output(output: &str) -> Vec<Diagnostic> {
        let mut res = Vec::new();
        for line in output.lines() {
            let Ok(json_val) = serde_json::from_str::<serde_json::Value>(line) else { continue };
            let Some(reason) = json_val.get("reason") else { continue };
            if reason.as_str() == Some("compiler-message") {
                let Some(message) = json_val.get("message").and_then(parse_message) else {
                    continue;
                };
                let target: Option<DiagnosticTarget> =
                    json_val.get("target").and_then(parse_target);
                let package_id = json_val.get("package_id").map(|id| id.to_string());
                res.push(Diagnostic { message, package_id, target })
            }
        }
        res
    }

    fn parse_lemma(message: &serde_json::Value) -> Option<Lemma> {
        tracing::info!("{message}");
        let name = message.get("lemma_name")?.as_str()?.to_string();
        let file_name = message.get("file_name")?.as_str()?.to_string();
        let start_line = message.get("start_line")?.as_i64()?;
        let end_line = message.get("end_line")?.as_i64()?;
        let start_col = message.get("start_col")?.as_i64()?;
        let end_col = message.get("end_col")?.as_i64()?;
        Some(Lemma { name, file_name, start_line, start_col, end_line, end_col })
    }

    fn parse_flux_lemmas(output: &str) -> Vec<Lemma> {
        let mut res = Vec::new();
        tracing::info!("ABOUT TO PARSE LEMMAS");
        for line in output.lines() {
            let Ok(json_val) = serde_json::from_str::<serde_json::Value>(line) else { continue };
            let Some(reason) = json_val.get("reason") else { continue };
            if reason.as_str() == Some("compiler-message") {
                let Some(lemma) = json_val.get("message").and_then(Self::parse_lemma) else { continue };
                res.push(lemma);
            }
        }
        res
    }

    pub async fn verify_repository(&self, repo_path: &str) -> Result<VerificationReport, String> {
        let mut cmd = Self::flux_command(repo_path, None, None);
        tracing::info!("About to execute command {:?}", cmd);
        let mut child = cmd
            .spawn()
            .map_err(|_| "Failed to run Flux process".to_string())?;
        let stdout = child
            .stdout
            .take()
            .map(Ok)
            .unwrap_or(Err("Failed to capture stdout from Flux process".to_string()))?;
        let mut output = String::new();
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line.map_err(|err| format!("Failed to read output: {err}"))?;
            output.push_str(&line);
            output.push('\n');
        }
        let status = child
            .wait()
            .map_err(|err| format!("Process wait failed: {err}"))?;
        let diagnostics = Self::parse_flux_output(&output);

        Ok(VerificationReport { success: status.success(), diagnostics })
    }

    pub async fn verify_package(&self, repo_path: &str, packages: Option<&[&str]>) -> Result<VerificationReport, String> {
        let mut cmd = Self::flux_command(repo_path, packages, None);
        tracing::info!("About to execute command {:?}", cmd);
        let mut child = cmd
            .spawn()
            .map_err(|_| "Failed to run Flux process".to_string())?;
        let stdout = child
            .stdout
            .take()
            .map(Ok)
            .unwrap_or(Err("Failed to capture stdout from Flux process".to_string()))?;
        let mut output = String::new();
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line.map_err(|err| format!("Failed to read output: {err}"))?;
            output.push_str(&line);
            output.push('\n');
        }
        let status = child
            .wait()
            .map_err(|err| format!("Process wait failed: {err}"))?;
        let diagnostics = Self::parse_flux_output(&output);

        Ok(VerificationReport { success: status.success(), diagnostics })
    }

    pub async fn get_lemmas(&self, repo_path: &str) -> Result<Vec<Lemma>, String> {
        let flux_flags = ["-Fdump-lemmas"];
        let mut cmd = Self::flux_command(repo_path, None, Some(&flux_flags));
        tracing::info!("About to execute command {:?}", cmd);
        let mut child = cmd
            .spawn()
            .map_err(|_| "Failed to run Flux process".to_string())?;
        let stdout = child
            .stdout
            .take()
            .map(Ok)
            .unwrap_or(Err("Failed to capture stdout from Flux process".to_string()))?;
        let mut output = String::new();
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line.map_err(|err| format!("Failed to read ouptut: {err}"))?;
            output.push_str(&line);
            output.push('\n');
        }
        let status = child
            .wait()
            .map_err(|err| format!("Process wait failed: {err}"))?;
        let lemmas = Self::parse_flux_lemmas(&output);
        Ok(lemmas)
    }
    
}
