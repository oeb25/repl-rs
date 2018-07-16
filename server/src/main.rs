extern crate actix_web;
extern crate failure;
extern crate quote;
extern crate serde_json;
extern crate shared;
extern crate syn;
extern crate tempfile;
extern crate time;

#[macro_use]
extern crate serde_derive;

use shared::*;

use actix_web::middleware::cors::Cors;
use actix_web::{http, server, App, Json};
use quote::ToTokens;
use std::path::PathBuf;

struct Executioner {
    build_dir: PathBuf,
}

impl Executioner {
    fn new<P: Into<PathBuf>>(build_dir: P) -> Result<Executioner, failure::Error> {
        let build_dir: PathBuf = build_dir.into();
        let src_dir = build_dir.join("src");
        std::fs::create_dir_all(&src_dir)?;
        std::fs::write(
            build_dir.join("Cargo.toml"),
            r#"[package]
name = "runtree"
version = "0.1.0"

[dependencies]
"#,
        )?;
        std::fs::write(src_dir.join("main.rs"), "")?;
        Ok(Executioner { build_dir })
    }
    fn execute(&self, execution: &Execution) -> Result<ExecutionResult, XXError> {
        let build_dir = self
            .build_dir
            .canonicalize()
            .expect("failed to canonicalize build_dir");

        let run_dir = execution.run_dir.as_ref().unwrap_or(&build_dir);

        // Write to file
        let src = execution.prepare_src()?;

        std::fs::write(build_dir.join("src/main.rs"), src).expect("failed to write file");

        let start_time = time::PreciseTime::now();

        // Build
        let build_output = std::process::Command::new("cargo")
            .current_dir(&build_dir)
            .arg("build")
            .output()
            .expect("failed to build");
        let build_time = time::PreciseTime::now();
        if !build_output.status.success() {
            let error = String::from_utf8(build_output.stderr).unwrap();
            return Err(XXError::BuildError { error });
        }

        // Run
        let output = std::process::Command::new(build_dir.join("./target/debug/runtree"))
            .current_dir(run_dir)
            .output()
            .expect("failed to execute");
        let run_time = time::PreciseTime::now();

        let stdout = String::from_utf8(output.stdout).expect("failed to parse stdout to utf8");

        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr).unwrap();
            return Err(XXError::RunError { stdout, stderr });
        }

        // Parse output
        let x = stdout.rfind(']').expect("failed to parse debug output");
        let n: usize = stdout[x + 1..stdout.len() - 1].parse().unwrap();
        #[derive(Debug, Deserialize)]
        struct ParsedOutput(Vec<Option<String>>);
        let parsed_output: ParsedOutput = serde_json::from_str(&stdout[x - n - 1..x + 1]).unwrap();

        let fix_time = |t: time::Duration| (t.num_seconds() * 1000 + t.num_milliseconds()) as u32;

        Ok(ExecutionResult {
            build_time: fix_time(start_time.to(build_time)),
            run_time: fix_time(build_time.to(run_time)),
            nodes: parsed_output
                .0
                .into_iter()
                .map(|s| match s {
                    Some(s) => NodeResult::String(s),
                    None => NodeResult::None,
                })
                .collect(),
            stdout: (&stdout[0..x - n - 1]).to_string(),
        })
    }
}

struct Execution {
    run_dir: Option<PathBuf>,
    parameters: ExecutionParameters,
}

impl Execution {
    fn prepare_src(&self) -> Result<String, XXError> {
        let parsed_nodes = self
            .parameters
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let src = format!("{{{}}}", node.content);
                let parsed =
                    syn::parse_str::<syn::Block>(&src).map_err(|error| XXError::ParseNode {
                        src,
                        error: format!("{}", error),
                        node: i,
                    })?;
                Ok(parsed.stmts)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let code = parsed_nodes
            .into_iter()
            .map(|stmts| {
                let len = stmts.len();
                let mut did_push = false;
                let mut block = stmts
                    .into_iter()
                    .enumerate()
                    .map(|(i, stmt)| {
                        let src = match stmt {
                            syn::Stmt::Local(local) => format!("{}", local.into_token_stream()),
                            syn::Stmt::Item(item) => format!("{}", item.into_token_stream()),
                            syn::Stmt::Semi(expr, _) => format!("{};", expr.into_token_stream()),
                            syn::Stmt::Expr(expr) => if len - 1 == i {
                                did_push = true;
                                format!(
                                    "node_results.push(({}).to_debugable());",
                                    expr.into_token_stream()
                                )
                            } else {
                                panic!("expression found not at end of node");
                            },
                        };
                        src
                    })
                    .collect::<Vec<_>>();
                if !did_push {
                    block.push("node_results.push(\"null\".to_string());".to_string());
                }
                block.join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n");

        let src = format!(
            r#"
trait Debugable {{
    fn to_debugable(&self) -> String;
}}

impl<T> Debugable for T
    where T: std::fmt::Debug
{{
    fn to_debugable(&self) -> String {{
        format!("{{:?}}", format!("{{:?}}", self))
    }}
}}

fn main() {{
    let mut node_results = vec![];
    {{
        {}
    }}
    let out = node_results
        .into_iter()
        .map(|x| format!("{{}}", x))
        .collect::<Vec<_>>()
        .join(",");
    println!("[{{}}]{{}}", out, out.len());
}}
            "#,
            code
        );

        Ok(src)
    }
}

fn execute(
    (req, parameters): (actix_web::HttpRequest<AppState>, Json<ExecutionParameters>),
) -> Result<Json<ExecutionResponse>, failure::Error> {
    let state = req.state();
    println!("runninging {:?}", state.build_dir());
    let execution = Execution {
        run_dir: None,
        parameters: parameters.into_inner(),
    };
    let result = state.executioner.execute(&execution);
    Ok(Json(result))
}

#[allow(dead_code)]
struct AppState {
    build_tmp_dir: Option<tempfile::TempDir>,
    executioner: Executioner,
}

impl AppState {
    fn build_dir(&self) -> &std::path::Path {
        self.build_tmp_dir.as_ref().unwrap().path()
    }
}

fn main() {
    server::new(|| {
        let build_tmp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let executioner = Executioner::new(build_tmp_dir.path())
            .expect("failed to create executioner");
        App::with_state(AppState {
            build_tmp_dir: Some(build_tmp_dir),
            executioner,
        }).configure(|app| {
            Cors::for_app(app)
                .allowed_methods(vec!["GET", "POST"])
                .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                .allowed_header(http::header::CONTENT_TYPE)
                .max_age(3600)
                .resource("/execute", |r| r.with(execute))
                .register()
        })
    })
        .bind("127.0.0.1:8080")
        .unwrap()
        .run();
}
