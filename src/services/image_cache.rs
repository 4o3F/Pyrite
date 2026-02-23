use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::UNIX_EPOCH;

use image::GenericImageView;
use tracing::info;

use crate::models::{ContestState, TeamStatus};

const IMAGE_CACHE_MAGIC: &[u8] = b"PYRITE_AWARD_CACHE_V1";

#[derive(Clone)]
pub struct DecodedImageData {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

pub enum ImageCacheEvent {
    Started {
        total: usize,
    },
    Progress {
        completed: usize,
        total: usize,
    },
    Finished {
        completed: usize,
        total: usize,
        ok: usize,
        miss: usize,
    },
    Failed {
        message: String,
    },
}

pub fn resolve_fallback_path(raw: Option<&str>) -> Option<PathBuf> {
    let raw_path = raw?.trim();
    if raw_path.is_empty() {
        return None;
    }
    let path = PathBuf::from(raw_path);
    if path.exists() && path.is_file() {
        Some(path)
    } else {
        None
    }
}

pub fn image_cache_root(base_path: &Path) -> PathBuf {
    base_path.join(".pyrite_cache").join("image_cache")
}

pub fn image_cache_path_for_team(cache_root: &Path, team_id: &str, max_dimension: u32) -> PathBuf {
    cache_root.join(format!("team_{team_id}_{max_dimension}.bin"))
}

pub fn image_cache_path_for_source(
    cache_root: &Path,
    source_path: &Path,
    prefix: &str,
    max_dimension: u32,
) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source_path.to_string_lossy().hash(&mut hasher);
    let key = hasher.finish();
    cache_root.join(format!("{prefix}_{key:016x}_{max_dimension}.bin"))
}

pub fn decode_image_data_cached(
    source_path: &Path,
    max_dimension: u32,
    cache_path: &Path,
) -> Option<DecodedImageData> {
    let stamp = source_file_stamp(source_path)?;
    if let Some(cached) = try_load_cached_award_image(cache_path, stamp) {
        return Some(cached);
    }

    let decoded = decode_image_data(source_path, max_dimension)?;
    let _ = save_cached_award_image(cache_path, stamp, &decoded);
    Some(decoded)
}

pub fn collect_awarded_team_ids_bottom_to_top(contest_state: &ContestState) -> Vec<String> {
    let mut awarded_team_ids: HashSet<String> = HashSet::new();
    for award in contest_state.awards.values() {
        if award.citation.trim().is_empty() {
            continue;
        }
        for team_id in &award.team_ids {
            awarded_team_ids.insert(team_id.clone());
        }
    }
    order_team_ids_bottom_to_top(
        &contest_state.leaderboard_finalized,
        awarded_team_ids.into_iter().collect(),
    )
}

pub fn order_team_ids_bottom_to_top(finalized: &[TeamStatus], input_team_ids: Vec<String>) -> Vec<String> {
    let mut remaining: HashSet<String> = input_team_ids.into_iter().collect();
    let mut ordered: Vec<String> = Vec::with_capacity(remaining.len());

    for team in finalized.iter().rev() {
        if remaining.remove(&team.team_id) {
            ordered.push(team.team_id.clone());
        }
    }

    let mut extras: Vec<String> = remaining.into_iter().collect();
    extras.sort();
    ordered.extend(extras);
    ordered
}

pub fn spawn_image_cache_precompute(
    base_path: PathBuf,
    team_ids: Vec<String>,
    team_photo_extension: String,
    fallback_path: Option<PathBuf>,
    max_dimension: u32,
) -> Receiver<ImageCacheEvent> {
    let (tx, rx) = mpsc::channel::<ImageCacheEvent>();
    let cache_root = image_cache_root(&base_path);
    let ext = team_photo_extension.trim().trim_start_matches('.').to_string();

    thread::spawn(move || {
        if ext.is_empty() {
            let _ = tx.send(ImageCacheEvent::Failed {
                message: "team_photo_extension is empty".to_string(),
            });
            return;
        }

        let total = team_ids.len() + usize::from(fallback_path.is_some());
        let _ = tx.send(ImageCacheEvent::Started { total });
        let worker_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(1, 4);
        let max_jobs = worker_threads.max(1);

        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(worker_threads)
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(err) => {
                let _ = tx.send(ImageCacheEvent::Failed {
                    message: format!("failed to initialize precompute runtime: {err}"),
                });
                return;
            }
        };

        let tx_progress = tx.clone();
        let (ok, miss, completed) = runtime.block_on(async move {
            let mut ok = 0usize;
            let mut miss = 0usize;
            let mut completed = 0usize;

            if let Some(path) = fallback_path {
                let cache_path =
                    image_cache_path_for_source(&cache_root, &path, "fallback", max_dimension);
                let handle = tokio::task::spawn_blocking(move || {
                    decode_image_data_cached(&path, max_dimension, &cache_path).is_some()
                });
                let fallback_ok = handle.await.unwrap_or(false);
                if fallback_ok {
                    ok += 1;
                } else {
                    miss += 1;
                }
                completed += 1;
                let _ = tx_progress.send(ImageCacheEvent::Progress { completed, total });
            }

            let mut handles = Vec::with_capacity(max_jobs);
            for team_id in team_ids {
                let team_id_for_task = team_id.clone();
                let base_path_for_task = base_path.clone();
                let cache_root_for_task = cache_root.clone();
                let ext_for_task = ext.clone();
                let handle = tokio::task::spawn_blocking(move || {
                    let path = base_path_for_task
                        .join("teams")
                        .join(format!("{team_id_for_task}.{ext_for_task}"));
                    if !path.exists() || !path.is_file() {
                        return false;
                    }
                    let cache_path =
                        image_cache_path_for_team(&cache_root_for_task, &team_id_for_task, max_dimension);
                    decode_image_data_cached(&path, max_dimension, &cache_path).is_some()
                });
                handles.push(handle);

                if handles.len() >= max_jobs {
                    let handle = handles.remove(0);
                    let team_ok = handle.await.unwrap_or(false);
                    if team_ok {
                        ok += 1;
                    } else {
                        miss += 1;
                    }
                    completed += 1;
                    let _ = tx_progress.send(ImageCacheEvent::Progress { completed, total });
                }
            }

            for handle in handles {
                let team_ok = handle.await.unwrap_or(false);
                if team_ok {
                    ok += 1;
                } else {
                    miss += 1;
                }
                completed += 1;
                let _ = tx_progress.send(ImageCacheEvent::Progress { completed, total });
            }

            (ok, miss, completed)
        });

        info!(
            "Award cache precompute finished: completed={}, ok={}, miss={}",
            completed, ok, miss
        );
        let _ = tx.send(ImageCacheEvent::Finished {
            completed,
            total,
            ok,
            miss,
        });
    });

    rx
}

fn source_file_stamp(path: &Path) -> Option<(u64, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    let file_len = meta.len();
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some((file_len, mtime))
}

fn try_load_cached_award_image(
    cache_path: &Path,
    expected_stamp: (u64, u64),
) -> Option<DecodedImageData> {
    let mut file = std::fs::File::open(cache_path).ok()?;
    let mut magic = vec![0u8; IMAGE_CACHE_MAGIC.len()];
    file.read_exact(&mut magic).ok()?;
    if magic != IMAGE_CACHE_MAGIC {
        return None;
    }

    let width = read_u32_le(&mut file)? as usize;
    let height = read_u32_le(&mut file)? as usize;
    let src_len = read_u64_le(&mut file)?;
    let src_mtime = read_u64_le(&mut file)?;
    if (src_len, src_mtime) != expected_stamp {
        return None;
    }

    let pixel_len = width.checked_mul(height)?.checked_mul(4)?;
    let mut rgba = vec![0u8; pixel_len];
    file.read_exact(&mut rgba).ok()?;

    Some(DecodedImageData {
        width,
        height,
        rgba,
    })
}

fn save_cached_award_image(
    cache_path: &Path,
    stamp: (u64, u64),
    image: &DecodedImageData,
) -> std::io::Result<()> {
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(cache_path)?;
    file.write_all(IMAGE_CACHE_MAGIC)?;
    file.write_all(&(image.width as u32).to_le_bytes())?;
    file.write_all(&(image.height as u32).to_le_bytes())?;
    file.write_all(&stamp.0.to_le_bytes())?;
    file.write_all(&stamp.1.to_le_bytes())?;
    file.write_all(&image.rgba)?;
    Ok(())
}

fn read_u32_le(file: &mut std::fs::File) -> Option<u32> {
    let mut buf = [0u8; 4];
    file.read_exact(&mut buf).ok()?;
    Some(u32::from_le_bytes(buf))
}

fn read_u64_le(file: &mut std::fs::File) -> Option<u64> {
    let mut buf = [0u8; 8];
    file.read_exact(&mut buf).ok()?;
    Some(u64::from_le_bytes(buf))
}

pub fn decode_image_data(path: &Path, max_dimension: u32) -> Option<DecodedImageData> {
    let bytes = std::fs::read(path).ok()?;
    let mut decoded = image::load_from_memory(&bytes).ok()?;
    let (width, height) = decoded.dimensions();
    let max_side = width.max(height);
    if max_side > max_dimension {
        decoded = decoded.resize(
            max_dimension,
            max_dimension,
            image::imageops::FilterType::Triangle,
        );
    }
    let rgba = decoded.to_rgba8();
    Some(DecodedImageData {
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        rgba: rgba.into_raw(),
    })
}
