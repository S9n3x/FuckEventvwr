use super::utils::{filter_event_by_name, parse_event_id};
use crate::cfg::sturct::Config;
use crate::parser::definition::EventRecord;
use crate::parser::xml::XmlParser;
use evtx::EvtxParser;
use std::collections::{HashSet, VecDeque};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::task;

const BATCH_SIZE: usize = 500;

pub struct ParseSummary {
    pub scanned: usize,
    pub matched: usize,
}

/// 查找当前模型需要的日志文件，并在开始解析前验证它们可读取。
/// 该预检用于在普通权限不足时触发一次按需提权，而非无条件请求管理员权限。
pub fn required_evtx_files(cfg: &Config) -> io::Result<Vec<PathBuf>> {
    let path = &cfg.path;
    let metadata = fs::metadata(path)?;
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("日志路径不是目录：{}", path.display()),
        ));
    }

    let all_files = list_evtx(path)?;
    let files = filter_event_by_name(all_files, &cfg.model.get_model_rule().files);
    if files.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "在目录 {} 中未找到 {} 模块需要的 EVTX 日志文件",
                path.display(),
                cfg.model.command_name()
            ),
        ));
    }

    for file in &files {
        File::open(file)?;
    }

    Ok(files)
}

pub async fn run_parser(cfg: Config, files: Vec<PathBuf>) -> ParseSummary {
    let rule = &cfg.model.get_model_rule();

    // 提高匹配效率
    let target_ids: Arc<HashSet<u16>> = Arc::new(rule.ids.clone().into_iter().collect());

    // 通道
    let (tx, mut rx) = mpsc::channel(256);

    // Arc+Mutex 安全共享计数器
    let total_num = Arc::new(Mutex::new(0u64));
    let mut handles = vec![];

    for fp in files {
        let total_counter = Arc::clone(&total_num);
        let ids = Arc::clone(&target_ids);

        let tx_clone = tx.clone();

        let handle = task::spawn_blocking(move || {
            let mut total_num = 0;
            if let Ok(mut parser) = EvtxParser::from_path(fp) {
                for record in parser.records() {
                    if let Ok(r) = record {
                        let xml = &r.data;

                        if let Some(id) = parse_event_id(xml) {
                            if ids.contains(&id) {
                                let _ = tx_clone.blocking_send(xml.clone());
                            }
                        }
                    }
                    total_num += 1;
                }
            }
            let mut num = total_counter.lock().unwrap();
            *num += total_num;
        });

        handles.push(handle);
    }

    // 批量并行解析
    drop(tx);
    let max_inflight = cfg.jobs;
    let collect_handle = tokio::spawn(async move {
        let mut batch: Vec<String> = Vec::with_capacity(BATCH_SIZE);
        let mut inflight: VecDeque<task::JoinHandle<Vec<Box<dyn EventRecord + Send>>>> =
            VecDeque::new();
        let mut results: Vec<Box<dyn EventRecord + Send>> = Vec::new();

        while let Some(item) = rx.recv().await {
            batch.push(item);
            if batch.len() < BATCH_SIZE {
                continue;
            }
            if inflight.len() >= max_inflight {
                if let Some(h) = inflight.pop_front() {
                    if let Ok(parsed) = h.await {
                        results.extend(parsed);
                    }
                }
            }
            let model = cfg.model.clone();
            let batch_data = std::mem::take(&mut batch);
            inflight.push_back(task::spawn_blocking(move || {
                batch_data
                    .into_iter()
                    .map(|xml| model.parse(&xml))
                    .collect::<Vec<_>>()
            }));
        }

        if !batch.is_empty() {
            let model = cfg.model.clone();
            inflight.push_back(task::spawn_blocking(move || {
                batch
                    .into_iter()
                    .map(|xml| model.parse(&xml))
                    .collect::<Vec<_>>()
            }));
        }

        while let Some(h) = inflight.pop_front() {
            if let Ok(parsed) = h.await {
                results.extend(parsed);
            }
        }
        results
    });

    for handle in handles {
        let _ = handle.await;
    }

    // 导出模块
    let scanned = *total_num.lock().unwrap() as usize;
    let matched = crate::out::export::run(collect_handle.await.unwrap_or_default(), cfg.format);
    ParseSummary { scanned, matched }
}

pub fn list_evtx(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("evtx"))
        {
            files.push(path);
        }
    }

    Ok(files)
}
