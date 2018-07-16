#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeResult {
    String(String),
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub build_time: u32,
    pub run_time: u32,
    pub nodes: Vec<NodeResult>,
    pub stdout: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionNode {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionParameters {
    pub nodes: Vec<ExecutionNode>,
}

#[derive(Debug, Fail, Serialize, Deserialize)]
pub enum XXError {
    #[fail(
        display = "Failed to parse node.\n  Error:\n{}\n  Source:\n{}",
        error,
        src
    )]
    ParseNode {
        src: String,
        error: String,
        node: usize,
    },
    #[fail(
        display = "Build Error.\n  'cargo build' failed with output:\n{}",
        error
    )]
    BuildError { error: String },
    #[fail(
        display = "Run Error.\n  Running the program failed with output:\n{}",
        stderr
    )]
    RunError { stdout: String, stderr: String },
}

pub type ExecutionResponse = Result<ExecutionResult, XXError>;