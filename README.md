# Curly

Curly is a Rust utility for turning an entire code repository into a prompt that a Large Language Model can understand. It also reinjects the model's answers back into the same repository. Use Curly when you want a repeatable way to ship code in and out of an LLM without manually copying files.

## Why Curly?

- **One command prompt generation** – walk your project, honor `.gitignore`, and produce a single text file ready for an LLM.
- **Docs‐only extraction** – gather just the docstrings or comments so the model focuses on high level documentation.
- **Jupyter notebook support** – include cell outputs when you need them.
- **Safe injection** – Curly reads the LLM output and places each code block into the correct file.

## Install

1. Install a [Rust toolchain](https://www.rust-lang.org/tools/install).
2. Build Curly from source:

    ```bash
    git clone https://github.com/labiium/curly.git
    cd curly
    cargo build --release
    ```

The executable will be at `target/release/curly`.

Or

    ```bash
    cargo install --git https://github.com/labiium/curly
    ```
and the `curly` command will then be available.

## Command overview

Curly exposes two main subcommands. You can also invoke named configurations stored in a `curly.yaml` file.

### Generate

Create a prompt file from a repository.

- Basic run

    ```bash
    curly generate --path my_project --language rust --output repo.out
    ```

- Important flags
    - `--ignore <pattern>` – repeat to skip files or directories.
    - `--docs-comments-only` – extract docstrings and comments without source code.
    - `--delimiter <token>` – fence used around each block (defaults to ```` ``` ````).

### Inject

Read an LLM's output and write changes back into the repository.

- Basic run

    ```bash
    curly inject --input llm_output.txt --path my_project
    ```

The input file should contain a path followed by a fenced code block for every change:

    ```
    src/main.rs
    ```rust
    fn main() {
        println!("hello updated world");
    }
    ```
    ```

### Running with a configuration

A `curly.yaml` file allows named setups. A minimal example is below.

```yaml
base:
  path: ./my_project
  language: rust
  output: prompt.out
  ignore:
    - target
  prompts:
    - "Summarise the project before rewriting."
```

Execute that configuration simply by running:

```bash
curly base
```

## Example output

Running `curly generate` on a small project might produce something like:

```text
sample_project_1
├── README.md
└── main.py

```README.md
# Sample Project 1
```

```main.py
print("Hello, world!")
```
```

This output can be fed directly into your favourite LLM. After editing, feed the modified file back to `curly inject`.

## Library usage

Curly can also be used as a library. The `Generator` and `Injector` types expose the same logic as the CLI:

```rust
use curly::{Config, Generator, Injector, run_and_write};

let config = Config { path: Some("my_project".into()), ..Default::default() };
let generator = Generator::default();
run_and_write(&generator, &config)?;
```

---

Curly streamlines the tedious parts of preparing code for an LLM and applying the results. Give it a try on your next project and focus on the creative parts instead of file management.
