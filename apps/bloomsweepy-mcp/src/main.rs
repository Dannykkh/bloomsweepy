use std::process::ExitCode;

use bloomsweepy_mcp::{Arguments, Command, execute, mcp};
use clap::Parser;

#[tokio::main]
async fn main() -> ExitCode {
    let arguments = Arguments::parse();
    let command = match arguments.command {
        Command::Mcp => {
            return match mcp::run_stdio().await {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => {
                    eprintln!("BroomSweepy MCP 표준 입출력 연결을 종료했습니다.");
                    ExitCode::from(4)
                }
            };
        }
        command => command,
    };
    let output = execute(command);
    let exit_code = output.exit_code();
    match serde_json::to_string(&output) {
        Ok(json) => println!("{json}"),
        Err(_) => {
            println!(
                "{{\"status\":\"error\",\"error\":{{\"code\":\"serialization_failed\",\"message\":\"결과를 JSON으로 만들지 못했습니다.\",\"retryable\":false}}}}"
            );
            return ExitCode::from(3);
        }
    }
    ExitCode::from(exit_code)
}
