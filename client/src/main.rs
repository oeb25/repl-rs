#![feature(nll)]

#[macro_use]
extern crate yew;
extern crate shared;
#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate failure;
extern crate serde;
extern crate serde_json;

use failure::Error;
use shared::*;
use yew::format::Json;
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::services::ConsoleService;

fn is_keyword(ident: &str) -> bool {
    match ident {
        "_" | "abstract" | "alignof" | "as" | "become" | "box" | "break" | "catch" | "const"
        | "continue" | "crate" | "default" | "do" | "else" | "enum" | "extern" | "false"
        | "final" | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "macro" | "match"
        | "mod" | "move" | "mut" | "offsetof" | "override" | "priv" | "proc" | "pure" | "pub"
        | "ref" | "return" | "self" | "sizeof" | "static" | "struct" | "super" | "trait"
        | "true" | "type" | "typeof" | "union" | "unsafe" | "unsized" | "use" | "virtual"
        | "where" | "while" | "yield" => true,
        _ => false,
    }
}

fn hightlight(src: &str) -> Html<Model> {
    enum State {
        Spaces(usize),
        Ident(usize),
        Int(usize),
        Float(usize),
        String(usize, bool, bool),
    }

    impl State {
        fn finish(self, src: &str, i: usize) -> Html<Model> {
            match self {
                State::Spaces(j) => html!{{&src[j..i]}},
                State::Ident(j) => {
                    let ident = &src[j..i];
                    html!{
                        <span class=if is_keyword(ident) {
                            "sh-ident sh-keyword"
                        } else {
                            "sh-ident"
                        },>
                            {ident}
                        </span>
                    }
                }
                State::Int(j) => {
                    html!{
                        <span class="sh-int",>
                            {&src[j..i]}
                        </span>
                    }
                }
                State::Float(j) => {
                    html!{
                        <span class="sh-float",>
                            {&src[j..i]}
                        </span>
                    }
                }
                State::String(j, _, _) => {
                    html!{
                        <span class="sh-string",>
                            {&src[j..i]}
                        </span>
                    }
                }
            }
        }
    }

    let mut state = State::Spaces(0);
    let mut tokens = vec![];

    for (i, c) in src.chars().enumerate() {
        let start_new = match &state {
            State::Spaces(_) => match c {
                c if c.is_alphabetic() => {
                    // tokens.push(state.finish(src, i));
                    // state = State::Ident(i);
                    true
                }
                c if c.is_numeric() => {
                    // tokens.push(state.finish(src, i));
                    // state = State::Int(i);
                    true
                }
                _ => true,
            },
            State::Ident(_) => match c {
                c if c.is_alphanumeric() => {
                    // ...
                    false
                }
                c if c.is_whitespace() => {
                    // tokens.push(state.finish(src, i));
                    // state = State::Spaces(i);
                    true
                }
                _ => {
                    // tokens.push(state.finish(src, i));
                    // state = State::Spaces(i);
                    true
                }
            },
            State::Int(j) => match c {
                c if c.is_numeric() || c == '_' => {
                    // ...
                    false
                }
                '.' => {
                    state = State::Float(*j);
                    false
                }
                c if c.is_alphabetic() => {
                    // tokens.push(state.finish(src, i));
                    // state = State::Ident(i);
                    true
                }
                _ => {
                    // tokens.push(state.finish(src, i));
                    // state = State::Spaces(i);
                    true
                }
            },
            State::Float(_) => match c {
                c if c.is_numeric() => {
                    // ...
                    false
                }
                c if c.is_alphanumeric() => {
                    // tokens.push(state.finish(src, i));
                    // state = State::Ident(i);
                    true
                }
                _ => {
                    // tokens.push(state.finish(src, i));
                    // state = State::Spaces(i);
                    true
                }
            },
            State::String(_, _, true) => true,
            State::String(j, true, _) => {
                state = State::String(*j, false, false);
                false
            }
            State::String(j, false, _) => match c {
                '\\' => {
                    state = State::String(*j, true, false);
                    false
                }
                '"' => {
                    state = State::String(*j, false, true);
                    false
                },
                _ => false,
            },
        };

        if start_new {
            tokens.push(state.finish(src, i));

            state = match c {
                '"' => State::String(i, false, false),
                c if c.is_alphabetic() => State::Ident(i),
                c if c.is_numeric() => State::Int(i),
                _ => State::Spaces(i),
            };
        }
    }

    let i = src.len();
    tokens.push(state.finish(src, i));

    html!{{for tokens}}
}

struct Node {
    code: String,
    result: Option<NodeResult>,
}

impl Node {
    fn new(code: String) -> Node {
        Node { code, result: None }
    }

    fn view<F, G>(&self, change: F, run: G) -> Html<Model>
    where
        F: 'static + Fn(String) -> Msg,
        G: 'static + Fn() -> Msg,
    {
        let output = match &self.result {
            None => html!{{""}},
            Some(NodeResult::None) => html!{<span class="faded-text",>{"none"}</span>},
            Some(NodeResult::String(s)) => html!{{s}},
        };

        let rows = self
            .code
            .chars()
            .fold(1, |acc, c| if c == '\n' { acc + 1 } else { acc });

        html! {
            <div class="node",>
                <div class="editor-row",>
                    <textarea
                        spellcheck="false",
                        value={&self.code},
                        oninput=|e| change(e.value),
                        onkeydown=|e| {
                            if e.shift_key() && &e.code() == "Enter" {
                                js!{@{e}.preventDefault()};
                                run()
                            } else {
                                Msg::Noop
                            }
                        },
                        rows=rows,
                    />
                    <code>
                        {hightlight(&self.code)}
                    </code>
                </div>
                <code class="output",>{output}</code>
            </div>
        }
    }
}

enum ExecutionState {
    Idle,
    Running(FetchTask),
    Done(ExecutionResponse),
}

pub struct Model {
    console: ConsoleService,
    web: FetchService,
    callback: Callback<Result<ExecutionResponse, Error>>,
    nodes: Vec<Node>,
    state: ExecutionState,
}

pub enum Msg {
    Noop,
    Bulk(Vec<Msg>),
    Run,
    AddNode(String),
    Response(ExecutionResponse),
    ChangeNode(usize, String),
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Model {
            console: ConsoleService::new(),
            web: FetchService::new(),
            callback: link.send_back(|res: Result<_, _>| Msg::Response(res.unwrap())),
            nodes: vec![Node {
                code: "let x = 12;\nx + 21".to_string(),
                result: None,
            }],
            state: ExecutionState::Idle,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Noop => {}
            Msg::Bulk(list) => for msg in list {
                self.update(msg);
                self.console.log("Bulk action");
            },
            Msg::Run => {
                self.console.log("Run!");
                let params = ExecutionParameters {
                    nodes: self
                        .nodes
                        .iter()
                        .map(|node| ExecutionNode {
                            content: node.code.clone(),
                        })
                        .collect(),
                };
                let task = run(&params, &mut self.web, self.callback.clone());
                self.state = ExecutionState::Running(task);
            }
            Msg::AddNode(code) => {
                self.nodes.push(Node::new(code));
            }
            Msg::Response(Ok(res)) => {
                for (node, res) in self.nodes.iter_mut().zip(&res.nodes) {
                    node.result = Some(res.clone());
                }
                self.state = ExecutionState::Done(Ok(res));
            }
            Msg::Response(Err(e)) => {
                self.state = ExecutionState::Done(Err(e));
            }
            Msg::ChangeNode(i, value) => {
                self.nodes[i].code = value;
            }
        }
        true
    }
}

fn run(
    execution_params: &ExecutionParameters,
    web: &mut FetchService,
    callback: Callback<Result<ExecutionResponse, Error>>,
) -> FetchTask {
    let handler = move |response: Response<Json<Result<ExecutionResponse, Error>>>| {
        let (meta, Json(data)) = response.into_parts();
        if meta.status.is_success() {
            callback.emit(data)
        } else {
            callback.emit(Err(format_err!("Nooooo!")))
        }
    };
    let request = Request::post("http://localhost:8080/execute")
        .header("content-type", "application/json")
        .body(Json(execution_params))
        .unwrap();
    web.fetch(request, handler.into())
}

impl Renderable<Model> for Model {
    fn view(&self) -> Html<Self> {
        let nodes = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| node.view(move |value| Msg::ChangeNode(i, value), || Msg::Run));

        let stats = match &self.state {
            ExecutionState::Done(Ok(res)) => {
                html!{
                    <div class="stats",>
                        <div class="stat",>
                            <div>{"Build Time"}</div>
                            <div>{format!("{}ms", res.build_time)}</div>
                        </div>
                        <div class="stat",>
                            <div>{"Run Time"}</div>
                            <div>{format!("{}ms", res.run_time)}</div>
                        </div>
                    </div>
                }
            }
            _ => html!{<div class="stats",/>},
        };

        html! {
            <div id="app", class=match self.state {
                ExecutionState::Idle => "idle",
                ExecutionState::Running(_) => "running",
                ExecutionState::Done(Ok(_)) => "ok",
                ExecutionState::Done(Err(_)) => "error",
            },>
                <div class="nodes",>{for nodes}</div>
                <div class="controls",>
                    <button onclick=|_| Msg::Run,>{"Run"}</button>
                    <button onclick=|_| Msg::AddNode(String::new()),>{"Add Node"}</button>
                </div>
                <pre class="stdoutput",>
                    <code>{match &self.state {
                        ExecutionState::Idle => html!{{""}},
                        ExecutionState::Running(_) => html!{{"Running..."}},
                        ExecutionState::Done(Ok(res)) => if res.stdout == "" {
                            html!{<span class="faded-text",>{"none"}</span>}
                        } else {
                            html!{{&res.stdout}}
                        },
                        ExecutionState::Done(Err(e)) => html!{{format!("{}", e)}},
                    }}</code>
                </pre>
                {stats}
            </div>
        }
    }
}

fn main() {
    yew::initialize();
    App::<Model>::new().mount_to_body();
    yew::run_loop();
}
