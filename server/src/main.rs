extern crate actix_web;
extern crate failure;
extern crate quote;
extern crate serde_json;
extern crate shared;
extern crate syn;
extern crate time;

#[macro_use]
extern crate serde_derive;

use shared::*;

use actix_web::middleware::cors::Cors;
use actix_web::{http, server, App, Json};
use quote::ToTokens;

struct Execution {
    parameters: ExecutionParameters,
}

impl Execution {
    fn run_dir(&self) -> std::path::PathBuf {
        "./runtree/".into()
    }
    fn write_to_project(&self) -> Result<(), XXError> {
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
                let out = node_results.into_iter().map(|x| format!("{{}}", x)).collect::<Vec<_>>().join(",");
                println!("[{{}}]{{}}", out, out.len());
            }}
        "#,
            code
        );

        std::fs::write(self.run_dir().join("src/main.rs"), src).expect("failed to write file");

        Ok(())
    }
    fn execute(&self) -> Result<ExecutionResult, XXError> {
        self.write_to_project()?;
        let start_time = time::PreciseTime::now();
        let build_output = std::process::Command::new("cargo")
            .current_dir(self.run_dir())
            .args(&[
                "build",
                "-Z",
                "unstable-options",
                "--out-dir",
                self.run_dir().join("./target").to_str().unwrap(),
            ])
            .output()
            .expect("failed to build");
        let build_time = time::PreciseTime::now();
        if !build_output.status.success() {
            let error = String::from_utf8(build_output.stderr).unwrap();
            return Err(XXError::BuildError { error });
        }
        let output = std::process::Command::new(self.run_dir().join("./target/runtree"))
            .current_dir(self.run_dir())
            .output()
            .expect("failed to execute");
        let run_time = time::PreciseTime::now();
        if !output.status.success() {
            let error = String::from_utf8(output.stderr).unwrap();
            return Err(XXError::RunError { error });
        }

        let out_str = String::from_utf8(output.stdout).expect("failed to parse stdout to utf8");

        let x = out_str.rfind(']').expect("failed to parse debug output");
        let n: usize = out_str[x + 1..out_str.len() - 1].parse().unwrap();
        #[derive(Debug, Deserialize)]
        struct ParsedOutput(Vec<Option<String>>);
        let parsed_output: ParsedOutput = serde_json::from_str(&out_str[x - n - 1..x + 1]).unwrap();

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
            stdout: (&out_str[0..x - n - 1]).to_string(),
        })
    }
}

fn execute(parameters: Json<ExecutionParameters>) -> Json<ExecutionResponse> {
    let parameters = parameters.into_inner();
    let execution = Execution { parameters };
    let result = execution.execute();
    Json(result)
}

fn main() {
    server::new(|| {
        App::new().configure(|app| {
            Cors::for_app(app)
                .allowed_methods(vec!["GET", "POST"])
                .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                .allowed_header(http::header::CONTENT_TYPE)
                .max_age(3600)
                .resource("/execute", |r| r.with(execute))
                .register()
        })
    }).bind("127.0.0.1:8080")
        .unwrap()
        .run();
}
