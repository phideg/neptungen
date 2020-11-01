use toml;

error_chain! {
    errors {}
    foreign_links {
        Io(::std::io::Error);
        Toml(toml::de::Error);
    }
}

pub fn print_message(e: &Error) {
    use term_painter::Color::*;
    use term_painter::ToStyle;
    println!("{}", Red.bold().paint(format!("error: {}", e)));

    for e in e.iter().skip(1) {
        println!("{}", Red.paint(format!("caused by: {}", e)));
    }

    // The backtrace is not always generated. Try to run this example
    // with `RUST_BACKTRACE=1`.
    if let Some(backtrace) = e.backtrace() {
        println!("backtrace: {:?}", backtrace);
    }
}

pub fn print_error(e: &Error) {
    use std::io::Write;
    use term_painter::Color::*;
    use term_painter::ToStyle;
    let stderr = &mut ::std::io::stderr();
    let errmsg = "Error writing to standard out";

    writeln!(stderr, "{}", Red.bold().paint(format!("error: {}", e))).expect(errmsg);

    for e in e.iter().skip(1) {
        writeln!(stderr, "{}", Red.paint(format!("caused by: {}", e))).expect(errmsg);
    }

    // The backtrace is not always generated. Try to run this example
    // with `RUST_BACKTRACE=1`.
    if let Some(backtrace) = e.backtrace() {
        writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
    }

    ::std::process::exit(1);
}
