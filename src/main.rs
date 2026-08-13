mod cfg;
mod cli;
mod models;
mod out;
mod parser;
mod privilege;

use cli::parser::*;
use parser::flow::{required_evtx_files, run_parser};
use std::path::PathBuf;

fn main() {
    let parsed = parser();
    let report_path = parsed.elevated_report.clone();
    let cfg = parsed.config;
    let model_name = cfg.model.command_name();
    let files = match required_evtx_files(&cfg) {
        Ok(files) => files,
        Err(error)
            if error.kind() == std::io::ErrorKind::PermissionDenied && !parsed.elevated_retry =>
        {
            eprintln!(
                "当前账户没有读取日志目录 {} 的权限，正在请求管理员权限…",
                cfg.path.display()
            );
            match privilege::restart_as_administrator() {
                Ok(result) => {
                    print_elevated_result(&result.message, result.exit_code);
                    std::process::exit(result.exit_code as i32);
                }
                Err(elevation_error) => {
                    eprintln!("无法以管理员权限重新启动：{elevation_error}");
                    std::process::exit(1);
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            finish(
                report_path,
                1,
                format!(
                    "仍无法读取日志目录 {}。请确认管理员账户对该目录及日志文件具有读取权限：{error}",
                    cfg.path.display()
                ),
            );
        }
        Err(error) => {
            finish(
                report_path,
                1,
                format!("无法读取日志目录 {}：{error}", cfg.path.display()),
            );
        }
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        // async 侧只有 collector 协程做轻量调度（channel 收发 + 批次分发），
        // 不需要更多 worker 线程。evtx 解码和 XML 解析都在 blocking 线程池，由 -j 控制。
        .worker_threads(4)
        .max_blocking_threads(cfg.jobs)
        .enable_all()
        .build()
        .unwrap();

    let start = std::time::Instant::now();
    let summary = rt.block_on(run_parser(cfg, files));
    let result = if summary.matched == 0 {
        format!(
            "总计扫描到的事件记录数: {}\n未发现符合 {} 模块规则的事件，未生成导出文件。\n总耗时: {:?}",
            summary.scanned,
            model_name,
            start.elapsed()
        )
    } else {
        format!(
            "总计扫描到的事件记录数: {}\n符合 {} 模块规则的事件数: {}\n总耗时: {:?}",
            summary.scanned,
            model_name,
            summary.matched,
            start.elapsed()
        )
    };
    finish(report_path, 0, result);
}

fn print_elevated_result(message: &str, exit_code: u32) {
    if exit_code == 0 {
        println!("{message}");
    } else {
        eprintln!("{message}");
    }
}

/// 普通运行时直接输出；管理员子进程运行时将摘要回传给等待它的原进程。
fn finish(report_path: Option<PathBuf>, exit_code: i32, message: String) -> ! {
    if let Some(path) = report_path {
        if let Err(error) = std::fs::write(&path, &message) {
            eprintln!("{message}");
            eprintln!("无法回传管理员进程结果到 {}：{error}", path.display());
        }
    } else if exit_code == 0 {
        println!("{message}");
    } else {
        eprintln!("{message}");
    }

    std::process::exit(exit_code);
}

#[cfg(test)]
mod test;
