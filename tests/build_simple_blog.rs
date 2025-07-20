use std::{env, path::PathBuf, process::Command};

#[test]
fn test_build() {
    // make sure we are in the project root
    let mut examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    env::set_current_dir(examples_dir.as_path()).unwrap();

    // first build simple_blog from scratch
    examples_dir.push("examples/simple_blog");
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--project-path",
            examples_dir.to_str().unwrap(),
            "build",
            "--clean",
        ])
        .output();
    assert!(output.is_ok());
    dbg!(&output);
    // unclear why result is written to stderr
    let output = output.unwrap().stderr;
    let output = String::from_utf8_lossy(output.as_ref());
    if cfg!(windows) {
        // on Windows, the path separator is a backslash
        assert!(output.contains("PUBLIC\\index.html"));
        assert!(output.contains("PUBLIC\\Galleries\\index.html"));
        assert!(output.contains("PUBLIC\\Tutorials\\Training\\index.html"));
        assert!(output.contains("PUBLIC\\Tutorials\\Feeding\\index.html"));
        assert!(output.contains("PUBLIC\\Posts\\Where does it come from\\index.html"));
        assert!(output.contains("PUBLIC\\Posts\\What is Lorem Ipsum\\index.html"));
    } else {
        // on Unix-like systems, the path separator is a forward slash
        assert!(output.contains("PUBLIC/index.html"));
        assert!(output.contains("PUBLIC/Galleries/index.html"));
        assert!(output.contains("PUBLIC/Tutorials/Training/index.html"));
        assert!(output.contains("PUBLIC/Tutorials/Feeding/index.html"));
        assert!(output.contains("PUBLIC/Posts/Where does it come from/index.html"));
        assert!(output.contains("PUBLIC/Posts/What is Lorem Ipsum/index.html"));
    }
}
