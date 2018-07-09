extern crate syn;
extern crate quote;
extern crate serde_json;
extern crate actix_web;
#[macro_use] extern crate serde_derive;
use quote::ToTokens;
use actix_web::middleware::cors::Cors;
use actix_web::{http, server, App, Responder, Json, fs, Path, HttpRequest, HttpResponse};

const ALL_THE_FILES: &[(&str, &str)] = &include!(concat!(env!("OUT_DIR"), "/all_the_files.rs"));

#[derive(Debug, Serialize)]
struct NodeResult {
	str: String,
}

#[derive(Debug, Serialize)]
struct ExecutionResult {
	nodes: Vec<NodeResult>,
	stdout: String,
}

#[derive(Debug, Deserialize)]
struct ExecutionNode {
	content: String
}

#[derive(Debug, Deserialize)]
struct ExecutionParameters {
	nodes: Vec<ExecutionNode>,
}

impl ExecutionParameters {
	fn run_dir(&self) -> std::path::PathBuf {
		"./runtree/".into()
	}
	fn write_to_project(&self) {
		let parsed_nodes = self.nodes.iter().map(|node| {
			let src = format!("{{{}}}", node.content);
			let parsed = syn::parse_str::<syn::Block>(&src)
				.expect(&format!("failed to parse node: {}", src));
			parsed.stmts
		}).collect::<Vec<_>>();

		let code = parsed_nodes.into_iter().map(|stmts| {
			let len = stmts.len();
			let mut did_push = false;
			let mut block = stmts.into_iter().enumerate().map(|(i, stmt)| {
				let src = match stmt {
					syn::Stmt::Local(local) => format!("{}", local.into_token_stream()),
					syn::Stmt::Item(item) => format!("{}", item.into_token_stream()),
					syn::Stmt::Semi(expr, _) => format!("{};", expr.into_token_stream()),
					syn::Stmt::Expr(expr) => if len - 1 == i {
						did_push = true;
						format!("node_results.push(({}).to_debugable());", expr.into_token_stream())
					} else {
						panic!("expression found not at end of node");
					},
				};
				src
			}).collect::<Vec<_>>();
			if !did_push {
				block.push("node_results.push(\"null\".to_string());".to_string());
			}
			block.join("\n")
		}).collect::<Vec<_>>().join("\n");

		let src = format!(r#"
			trait Debugable {{
				fn to_debugable(&self) -> String;
			}}

			impl<T> Debugable for T
				where T: std::fmt::Debug
			{{
				fn to_debugable(&self) -> String {{
					format!("{{:?}}", self)
				}}
			}}

			fn main() {{
				let mut node_results = vec![];
				{{
					{}
				}}
				let out = node_results.into_iter().map(|x| format!("{{:?}}", x)).collect::<Vec<_>>().join(",");
				println!("[{{}}]{{}}", out, out.len());
			}}
		"#, code);

		std::fs::write(self.run_dir().join("src/main.rs"), src).expect("failed to write file");
	}
	fn execute(&self) -> ExecutionResult {
		self.write_to_project();
		let output = std::process::Command::new("cargo")
			.arg("run")
			.current_dir(self.run_dir())
			.output()
			.expect("failed to execute");

		let out_str = String::from_utf8(output.stdout).expect("failed to parse stdout to utf8");

		let x = out_str.rfind(']').expect("failed to parse debug output");
		let n: usize = out_str[x + 1..out_str.len() - 1].parse().unwrap();
		#[derive(Debug, Deserialize)]
		struct ParsedOutput(Vec<String>);
		let parsed_output: ParsedOutput = serde_json::from_str(&out_str[x - n - 1..x + 1]).unwrap();

		ExecutionResult {
			nodes: parsed_output.0.into_iter().map(|str| NodeResult { str }).collect(),
			stdout: (&out_str[0..x - n - 1]).to_string(),
		}
	}
}

fn index(_: HttpRequest) -> impl Responder {
	match ALL_THE_FILES.iter().find(|(name, _)| {
		let path = std::path::Path::new(name);
		path.file_name().unwrap() == "index.html"
	}) {
		Some(x) => Ok(HttpResponse::with_body(http::StatusCode::OK, x.1)),
		None => Err(actix_web::error::ErrorBadRequest("fuck"))
	}
}
fn execute(info: Json<ExecutionParameters>) -> impl Responder {
	let result = info.execute();
	Json(result)
}

fn static_files(file: Path<String>) -> impl Responder {
	let file: &str = &file;
	match ALL_THE_FILES.iter().find(|(name, _)| {
		let path = std::path::Path::new(name);
		path.file_name().unwrap() == file
	}) {
		Some(x) => HttpResponse::with_body(http::StatusCode::OK, x.1),
		None => HttpResponse::NotFound().finish(),
	}
}

fn main() {
	println!("{:?}", ALL_THE_FILES.iter().map(|(a, _)| a).collect::<Vec<_>>());
    server::new(
        || App::new()
        	.configure(|app| {
        		Cors::for_app(app)
	        		.allowed_methods(vec!["GET", "POST"])
	        		.allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
	        		.allowed_header(http::header::CONTENT_TYPE)
	        		.max_age(3600)
					.resource("/", |r| {
						r.method(http::Method::GET).with(index)
					})
					.resource("/execute", |r| {
						r.with(execute)
					})
					.resource("/{file}", |r| {
						r.method(http::Method::GET).with(static_files)
					})
	        		.register()
        	})
    )
        .bind("127.0.0.1:8080").unwrap()
        .run();
}