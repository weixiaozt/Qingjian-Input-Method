//! 按编码位置建立词图，保留有限条最佳路径；所有候选都由码表解释。
use super::entry::Entry;
use super::{Candidate, Request, Response, Segment};
use qingjian_core::sentence::LanguageModel;
use qingjian_lm::BigramModel;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

const BEAM: usize = 96;
const PER_CODE: usize = 32;
const RESULTS: usize = 12;

pub struct Decoder {
    table: HashMap<String, Vec<Entry>>,

    prefixes: HashSet<String>,

    model: BigramModel,

    total: f64,

    pub entries: usize,
}

impl Decoder {
    #[cfg(test)]
    pub(crate) fn test_fixture() -> Self {
        Self::from_parts(
            "你\twq\t100\n是\tj\t100\n谁\tywyg\t100\n",
            BigramModel::default(),
        )
        .expect("valid fixture")
    }

    pub fn load(data: &Path) -> Result<Self, String> {
        let source = std::fs::read_to_string(data.join("wubi.tsv")).map_err(|e| e.to_string())?;
        let model = BigramModel::from_path(&data.join("lm.qj")).map_err(|e| e.to_string())?;
        Self::from_parts(&source, model)
    }

    pub(super) fn from_parts(source: &str, model: BigramModel) -> Result<Self, String> {
        let mut table: HashMap<String, Vec<Entry>> = HashMap::new();
        let mut frequencies: HashMap<String, u32> = HashMap::new();
        let mut prefixes = HashSet::new();
        for line in source
            .lines()
            .filter(|s| !s.starts_with('#') && !s.is_empty())
        {
            let parts: Vec<_> = line.split('\t').collect();
            if parts.len() != 3 {
                return Err("invalid code table row".into());
            }
            let (text, code) = (parts[0], parts[1]);
            if code.is_empty() || code.len() > 4 || !code.bytes().all(|c| c.is_ascii_lowercase()) {
                continue;
            }
            let frequency = parts[2].parse::<u32>().map_err(|e| e.to_string())?.max(1);
            frequencies
                .entry(text.into())
                .and_modify(|f| *f = (*f).max(frequency))
                .or_insert(frequency);
            let entries = table.entry(code.into()).or_default();
            if !entries.iter().any(|e| e.text == text) {
                entries.push(Entry {
                    text: text.into(),
                    frequency,
                });
            }
            for length in 1..code.len() {
                prefixes.insert(code[..length].into());
            }
        }
        for entries in table.values_mut() {
            entries.sort_by(|a, b| b.frequency.cmp(&a.frequency).then(a.text.cmp(&b.text)));
        }
        let count = table.values().map(Vec::len).sum();
        let total = frequencies
            .values()
            .map(|&f| f as f64)
            .sum::<f64>()
            .max(1.0);
        Ok(Self {
            table,
            model,
            total,
            prefixes,
            entries: count,
        })
    }

    fn extend(&self, path: &Candidate, code: &str, entry: &Entry) -> Candidate {
        let previous = path.segments.last().map(|s| s.text.as_str());
        let probability = self
            .model
            .log_prob(previous, &entry.text)
            .unwrap_or_else(|| (entry.frequency as f64 / self.total).ln() - 1.0);
        let mut next = path.clone();
        next.text.push_str(&entry.text);
        next.segments.push(Segment {
            code: code.into(),
            text: entry.text.clone(),
        });
        // 一元简码串不能靠输出更多常用字取得优势；不针对任何测试句加分。
        next.score += probability - 0.8 / code.len() as f64;
        next
    }

    pub fn decode(&self, request: Request) -> Result<Response, String> {
        let now = Instant::now();
        let codes = request.codes.trim().to_ascii_lowercase();
        if codes.len() > 96 {
            return Err("实验版每次最多输入 96 个字母".into());
        }
        if !codes.bytes().all(|c| c.is_ascii_lowercase()) {
            return Err("请输入连续的 a–z 五笔编码，中间不需要空格".into());
        }
        let mut seed = Candidate::default();
        let mut offset = 0;
        for segment in request.locked {
            let entry = self
                .table
                .get(&segment.code)
                .and_then(|entries| entries.iter().find(|e| e.text == segment.text))
                .ok_or("锁定的词不属于该编码，请解除锁定后重试")?;
            if !codes[offset..].starts_with(&segment.code) {
                return Err("编码已修改，请解除原来的前词锁定".into());
            }
            offset += segment.code.len();
            seed = self.extend(&seed, &segment.code, entry);
        }
        let mut paths: Vec<Vec<Candidate>> = vec![Vec::new(); codes.len() + 1];
        paths[offset].push(seed);
        for position in offset..codes.len() {
            prune(&mut paths[position], BEAM);
            let current = std::mem::take(&mut paths[position]);
            // 末尾尚在输入的路径也保留，完整路径仍优先。
            if self.prefixes.contains(&codes[position..]) {
                for path in current.iter().take(RESULTS) {
                    let mut partial = path.clone();
                    partial.pending = codes[position..].into();
                    paths[codes.len()].push(partial);
                }
            }
            for length in 1..=4.min(codes.len() - position) {
                let code = &codes[position..position + length];
                let Some(entries) = self.table.get(code) else {
                    continue;
                };
                for path in &current {
                    if !path.pending.is_empty() {
                        continue;
                    }
                    for entry in entries.iter().take(PER_CODE) {
                        paths[position + length].push(self.extend(path, code, entry));
                    }
                }
                if paths[position + length].len() > BEAM * 24 {
                    prune(&mut paths[position + length], BEAM * 2);
                }
            }
        }
        let mut candidates = std::mem::take(&mut paths[codes.len()]);
        candidates.sort_by(|a, b| {
            a.pending
                .is_empty()
                .cmp(&b.pending.is_empty())
                .reverse()
                .then(b.score.total_cmp(&a.score))
                .then(a.text.cmp(&b.text))
        });
        let mut seen = HashSet::new();
        candidates.retain(|c| seen.insert((c.text.clone(), c.pending.clone())));
        candidates.truncate(RESULTS);
        if codes.is_empty() {
            candidates.clear();
        }
        let complete = candidates.first().is_some_and(|c| c.pending.is_empty());
        let message = if codes.is_empty() {
            "输入编码开始实验"
        } else if candidates.is_empty() {
            "这串编码没有完整路径，请检查编码或缩短后重试"
        } else if !complete {
            "末尾编码尚未完成，继续输入"
        } else {
            "切分由码表和词频模型推断；首选不对时可以换候选或锁定前词"
        };
        Ok(Response {
            codes,
            candidates,
            elapsed_ms: now.elapsed().as_secs_f64() * 1000.0,
            complete,
            message: message.into(),
        })
    }
}

fn prune(paths: &mut Vec<Candidate>, limit: usize) {
    paths.sort_by(|a, b| {
        a.pending
            .is_empty()
            .cmp(&b.pending.is_empty())
            .reverse()
            .then(b.score.total_cmp(&a.score))
            .then(a.text.cmp(&b.text))
    });
    let mut seen = HashSet::new();
    paths.retain(|p| {
        seen.insert((
            p.text.clone(),
            p.segments.last().map(|s| s.text.clone()),
            p.pending.clone(),
        ))
    });
    paths.truncate(limit);
}
