import * as monaco from "monaco-editor";

const $ = function<T extends string>(
	x: T,
	el: Document | HTMLElement = document
) {
	return el.querySelector(x);
};
const $$ = function<T extends string>(
	x: T,
	el: Document | HTMLElement = document
) {
	return Array.from(el.querySelectorAll(x) || []);
};
const keys = function<T>(a: T) {
	return Object.keys(a) as (keyof T)[];
};
const wait = (ms: number) => new Promise(res => setTimeout(() => res(), ms))
interface HOptions {
	className?: string;
	id?: string;
	width?: string | number;
	height?: string | number;
}

const h = function<K extends keyof HTMLElementTagNameMap>(
	name: K,
	opts: Partial<HTMLElementTagNameMap[K]> = {},
	children: HTMLElement[] | string = ""
) {
	const el = document.createElement(name);

	keys(opts).map(key => {
		el[key] = opts[key] as HTMLElementTagNameMap[K][typeof key];
	});

	if (typeof children == "string") {
		el.textContent = children;
	} else {
		for (const child of children) {
			el.appendChild(child);
		}
	}

	return el;
};

const EDITOR_STORAGE_KEY = "EDITOR_STORAGE_KEY";

type IdeStatus =
	| { type: "waiting" }
	| { type: "running" }
	| { type: "ok"; stdout: string }
	| { type: "error"; msg: string };

class Ide {
	container: HTMLElement;
	runButton: HTMLButtonElement;
	addNodeButton: HTMLButtonElement;
	nodesContainer: HTMLDivElement;
	nodes: CodeNode[] = [];
	controls: HTMLDivElement;
	stdoutput: HTMLElement;
	status: IdeStatus = { type: "waiting" };

	constructor(container: HTMLElement) {
		for (const child of Array.from(container.children)) {
			container.removeChild(child);
		}
		this.container = container;

		this.runButton = h(
			"button",
			{
				onclick: () => this.run()
			},
			"Run"
		);
		this.addNodeButton = h(
			"button",
			{
				onclick: () => this.addNode("")
			},
			"Add Node"
		);

		this.nodesContainer = h("div");
		this.controls = h("div", { className: "controls" }, [
			this.runButton,
			this.addNodeButton
		]);
		this.stdoutput = h("code");

		this.container.appendChild(this.nodesContainer);
		this.container.appendChild(this.controls);
		this.container.appendChild(
			h("pre", { className: "stdoutput" }, [this.stdoutput])
		);

		const stored = window.localStorage.getItem(EDITOR_STORAGE_KEY);
		const nodeContents = stored ? (JSON.parse(stored) as string[]) : [""];
		console.log(nodeContents);
		for (const content of nodeContents) {
			this.addNode(content);
		}
	}

	updateStatus(newStatus: IdeStatus) {
		this.container.classList.remove("running");
		this.status = newStatus;
		switch (this.status.type) {
			case "waiting":
				{
					this.runButton.textContent = "Run";
				}
				break;
			case "running":
				{
					this.runButton.textContent = "Running...";
					this.container.classList.add("running");
				}
				break;
			case "ok":
				{
					this.runButton.textContent = "Run";
					this.container.classList.remove("error");
					this.container.classList.add("ok");
					this.stdoutput.textContent = this.status.stdout;
				}
				break;
			case "error": {
				this.runButton.textContent = "Run";
				this.container.classList.remove("ok");
				this.container.classList.add("error");
				this.stdoutput.textContent = this.status.msg;
			}
		}
	}

	async run() {
		this.updateStatus({ type: "running" });
		const exeNodes: ExecutionNode[] = this.nodes.map(node => {
			node.output.textContent = "";
			return { content: node.value };
		});
		try {
			const result = await api.executeAll(exeNodes);
			console.assert(this.nodes.length == result.nodes.length);

			this.nodes.map((node, i) => {
				node.output.textContent = result.nodes[i].str;
			});

			this.updateStatus({ type: "ok", stdout: result.stdout });
		} catch (e) {
			this.updateStatus({ type: "error", msg: e });
			return;
		}
	}

	addNode(value: string) {
		const node = new CodeNode(value, () => this.run(), () => this.save());
		this.nodesContainer.appendChild(node.nodeDOM);
		this.nodes.push(node);
		node.layout();
	}

	save() {
		window.localStorage.setItem(
			EDITOR_STORAGE_KEY,
			JSON.stringify(this.nodes.map(node => node.value))
		);
	}
}

const EDITOR_LINE_HEIGHT = 20;

class CodeNode {
	nodeDOM: HTMLDivElement;
	editorContainer: HTMLDivElement;
	output: HTMLElement;
	editor: monaco.editor.IStandaloneCodeEditor;
	value: string;
	lineCount: number;

	constructor(value: string, run: () => void, save: () => void) {
		this.value = value;
		this.editorContainer = h("div", {
			className: "editor-container"
		});
		this.output = h("code", { className: "output" });
		this.nodeDOM = h(
			"div",
			{
				className: "node"
			},
			[this.editorContainer, this.output]
		);
		this.lineCount = -1;
		this.editor = monaco.editor.create(this.editorContainer, {
			language: "rust",
			minimap: {
				enabled: false
			},
			lineNumbers: "off",
			scrollBeyondLastLine: false,
			lineHeight: EDITOR_LINE_HEIGHT,
			value: this.value,
			scrollbar: {arrowSize: 0},
		});
		this.editor.onKeyDown(e => {
			save();
			if (e.shiftKey && e.keyCode == 3) {
				e.stopPropagation();
				e.preventDefault();
				run();
			}
		});
		this.editor.onKeyUp(e => {
			this.value = this.editor.getValue();
			this.layout();
		});
	}

	async layout() {
		const newCount = this.value.split(/\r\n|\r|\n/).length;
		if (this.lineCount !== newCount) {
			this.lineCount = newCount;
			this.editorContainer.style.height = `${(this.lineCount + 1) *
				EDITOR_LINE_HEIGHT}px`;

			console.log('pre')
			await wait(100);
			console.log('post')
			this.editor.layout();
		}
	}
}

const ide = new Ide($("#app") as HTMLElement);

interface ExecutionNode {
	content: string;
}

interface ExecutionNodeResult {
	str: string;
}

interface ExecutionResult {
	nodes: ExecutionNodeResult[];
	stdout: string;
}

class Api {
	url: string;

	constructor(url: string) {
		this.url = url;
	}

	fetch = async (path: string, body = {}) => {
		const res = await fetch(`${this.url}${path}`, {
			method: "POST",
			headers: {
				Accept: "application/json",
				"Content-Type": "application/json"
			},
			body: JSON.stringify(body)
		});
		return res.json();
	};

	executeAll = (nodes: ExecutionNode[]): Promise<ExecutionResult> =>
		this.fetch("/execute", { nodes });
}

const api = new Api("http://localhost:8080");
