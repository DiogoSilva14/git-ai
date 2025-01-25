extern crate getopts;
use anyhow::{anyhow, Result};
use futures::executor::block_on;
use getopts::Options;
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use std::{env, fs, io::stdin, process::Command};

const PROMPT: &str = r#"
The text above is a diff from a git commit, but it needs a commit message.
Write an appropriate commit message for the piece of code given above.
Make it a one-liner and do not output anything beside the commit message.


"#;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {}", program);
    print!("{}", opts.usage(&brief));
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut endpoint = env::var("GITAI_ENDPOINT").unwrap_or("http://localhost:11434".to_string());
    let mut model = env::var("GITAI_MODEL").unwrap_or("llama3.2".to_string());
    let mut verbose = false;

    let mut opts = Options::new();

    opts.optopt(
        "s",
        "",
        "set llama server endpoint. Defaults to http://localhost:11434",
        "ENDPOINT",
    );
    opts.optopt("m", "", "set llama model. Defaults to llama3.2", "MODEL");
    opts.optflag("h", "help", "print this help menu");
    opts.optflag("v", "verbose", "run program in verbose mode");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            panic!("{}", f.to_string())
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return Ok(());
    }

    if matches.opt_present("s") {
        endpoint = matches.opt_str("s").unwrap();
    }

    if matches.opt_present("m") {
        model = matches.opt_str("m").unwrap();
    }

    if matches.opt_present("v") {
        verbose = true;
    }

    if verbose {
        println!("Endpoint is {}", endpoint);
        println!("Model is {}", model);
        println!("Verbose is {}", verbose);
    }

    let output = Command::new("which")
        .arg("git")
        .output()
        .expect("failed to execute command");

    if !output.status.success() {
        return Err(anyhow!("git is not installed"));
    }

    let output = Command::new("git")
        .arg("status")
        .output()
        .expect("failed to execute command");

    if !output.status.success() {
        return Err(anyhow!("no git repository found"));
    }

    let output = Command::new("git")
        .arg("diff")
        .arg("--staged")
        .output()
        .expect("failed to execute command");

    if output.stdout.len() == 0 {
        println!("Staging is empty");
        return Ok(());
    }

    let mut prompt = String::from_utf8(output.stdout).expect("failed to retrieve command output");

    prompt.push_str(&PROMPT);

    if verbose {
        println!("Prompt: {}", prompt);
    }

    let ollama = Ollama::default();
    let mut commit_msg = block_on(ollama.generate(GenerationRequest::new(model, prompt)))
        .unwrap()
        .response;

    println!("Commit message");
    println!("=============================================================");
    println!("{}", commit_msg);
    println!("=============================================================");
    println!("Proceed? (c)ommit, (e)dit, (d)iscard");

    let mut choice = String::new();

    stdin()
        .read_line(&mut choice)
        .expect("unable to read user input");

    let choice = choice
        .strip_suffix("\r\n")
        .or(choice.strip_suffix("\n"))
        .unwrap_or(choice.as_str());

    match choice {
        "c" | "C" => {}
        "e" | "E" => {
            println!("Editing commit message");
            let tmp_file = String::from_utf8(
                Command::new("mktemp")
                    .output()
                    .expect("failed to create temporary file")
                    .stdout,
            )
            .expect("failed to retrieve command output");

            fs::write(tmp_file.clone(), commit_msg).expect("Unable to write to temporary file");

            Command::new(env::var("EDITOR").unwrap_or("vi".to_string()))
                .arg(tmp_file.clone())
                .spawn()
                .expect("Failed to spawn editor")
                .wait()
                .expect("Failed to wait for editor to close");

            commit_msg =
                fs::read_to_string(tmp_file).expect("Failed to read edited commit message");
        }
        "d" | "D" => {
            return Ok(());
        }
        _ => {
            return Err(anyhow!("Invalid option selected"));
        }
    }

    if verbose {
        println!("Commiting with message {}", commit_msg);
    }

    let out = String::from_utf8(
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(commit_msg.as_str())
            .output()
            .expect("Failed to commit")
            .stdout,
    )
    .expect("failed to retrieve command output");

    print!("{}", out);

    Ok(())
}
