use clap::Parser;

fn main() {
    let cli = statementbridge_cli::Cli::parse();
    let code = statementbridge_cli::run(cli);
    std::process::exit(code);
}
