# Curly


Use this command for the full usuage information

```bash
curly --help
```

# Generate

This is the default functionality of curly

```bash
curly --path my_lib --language rust > output.txt
```

This will convert the entire file directory into a prompt for the LLM and will ignore all irrelevant files of the given language you give

# Inject

Try use the [Curly GPT](https://chatgpt.com/g/g-1DjiUtEcZ-curly) or use the below default prompt.

Add this to your prompts to encourage the LLM to output the code in the correct format, make sure to include the examples:

```plain
- Provide the relative file path at the top of the block.
- Follow the relative file path with the code, ensuring there is no additional text in between.
- Think logically, breaking down the problem step by step within the comments of the code.

Example:

    `src/lib.rs`
    
    ```rust
    fn main() {
        println!("Hello, world!");
    }
    ```

    `src/main.py`

    ```python
    print("Hello, World!")
    ````

```

```bash
curly inject --path new_lib --input input.txt
```

