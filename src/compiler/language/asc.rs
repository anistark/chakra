use crate::compiler::builder::{BuildConfig, BuildResult, WasmBuilder};
use crate::error::CompilationResult;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct AssemblyScriptBuilder;

impl AssemblyScriptBuilder {
    pub fn new() -> Self {
        Self
    }
}

impl WasmBuilder for AssemblyScriptBuilder {
    fn language_name(&self) -> &str {
        "AssemblyScript"
    }

    fn entry_file_candidates(&self) -> &[&str] {
        &["assembly/index.ts", "package.json", "asconfig.json"]
    }

    fn supported_extensions(&self) -> &[&str] {
        &["ts"]
    }

    fn check_dependencies(&self) -> Vec<String> {
        let mut missing = Vec::new();

        if !self.is_tool_installed("node") {
            missing.push("node (Node.js - install from https://nodejs.org)".to_string());
        }

        if !self.is_tool_installed("npm") {
            missing.push("npm (Node Package Manager)".to_string());
        }

        missing
    }

    fn build(&self, config: &BuildConfig) -> CompilationResult<BuildResult> {
        let wasm_path = build_wasm(&config.project_path, &config.output_dir)
            .map_err(|e| crate::error::CompilationError::build_failed(self.language_name(), e))?;

        Ok(BuildResult {
            wasm_path,
            js_path: None,
            additional_files: vec![],
            is_wasm_bindgen: false,
        })
    }
}

/// Build a WASM file from an AssemblyScript project
pub fn build_wasm(project_path: &str, output_dir: &str) -> Result<String, String> {
    println!(
        "⚡️ Building WASM from AssemblyScript project at: {}",
        project_path
    );

    // Check if Node.js is installed
    let check_node = Command::new("node").arg("--version").output();

    if check_node.is_err() {
        return Err("Node.js is not installed or not in PATH. Please install Node.js.".to_string());
    }

    // Check if asc is installed (either globally or locally via npx)
    println!("⚙️ Building the project with AssemblyScript...");

    // Try to build with npx asc first
    let build_output = Command::new("npx")
        .current_dir(project_path)
        .args([
            "asc",
            "--optimize",
            "--outFile",
            "build/release.wasm",
            "assembly/index.ts",
        ])
        .output();

    let wasm_file = if let Ok(output) = build_output {
        if output.status.success() {
            Path::new(project_path).join("build/release.wasm")
        } else {
            // Try npm build command instead
            let npm_build = Command::new("npm")
                .current_dir(project_path)
                .args(["run", "asbuild"])
                .output()
                .map_err(|e| format!("Failed to build AssemblyScript project: {}", e))?;

            if !npm_build.status.success() {
                return Err(format!(
                    "Build failed: {}",
                    String::from_utf8_lossy(&npm_build.stderr)
                ));
            }

            // Look for build output files
            let build_dir = Path::new(project_path).join("build");
            let mut wasm_path = None;

            if build_dir.exists() {
                if let Ok(entries) = fs::read_dir(&build_dir) {
                    for entry in entries.flatten() {
                        if let Some(extension) = entry.path().extension() {
                            if extension == "wasm" {
                                wasm_path = Some(entry.path());
                                break;
                            }
                        }
                    }
                }
            }

            wasm_path.ok_or_else(|| "No WASM file found after build".to_string())?
        }
    } else {
        return Err(
            "AssemblyScript compiler not found. Make sure it's installed in your project."
                .to_string(),
        );
    };

    // Create output directory if it doesn't exist
    let output_path = Path::new(output_dir);
    fs::create_dir_all(output_path)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // Copy the wasm file to the output directory
    let output_file = output_path.join(wasm_file.file_name().unwrap());
    fs::copy(&wasm_file, &output_file).map_err(|e| format!("Failed to copy WASM file: {}", e))?;

    Ok(output_file.to_string_lossy().to_string())
}
